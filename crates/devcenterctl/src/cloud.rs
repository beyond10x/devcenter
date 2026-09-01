//! Idempotent cloud prerequisites for the optional hosted Vault profile.

use anyhow::{Context, Result, bail};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_eks::Client as EksClient;
use aws_sdk_iam::Client as IamClient;
use aws_sdk_kms::{Client as KmsClient, types::Tag};
use aws_sdk_s3::{
    Client as S3Client,
    types::{
        BucketLifecycleConfiguration, BucketLocationConstraint, BucketVersioningStatus,
        CreateBucketConfiguration, LifecycleExpiration, LifecycleRule, LifecycleRuleFilter,
        NoncurrentVersionExpiration, PublicAccessBlockConfiguration, ServerSideEncryption,
        ServerSideEncryptionByDefault, ServerSideEncryptionConfiguration, ServerSideEncryptionRule,
        VersioningConfiguration,
    },
};
use aws_sdk_sts::Client as StsClient;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone, Debug)]
pub struct EnsureVault {
    pub cluster_name: String,
    pub region: String,
    pub namespace: String,
    pub release: String,
    pub retention_days: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Output {
    kms_key_arn: String,
    vault_role_arn: String,
    backup_role_arn: String,
    backup_bucket: String,
    region: String,
}

#[allow(clippy::too_many_lines)] // One ordered idempotent transaction over five AWS APIs.
pub async fn ensure_vault(configuration: &EnsureVault) -> Result<()> {
    validate_name("cluster name", &configuration.cluster_name)?;
    validate_name("namespace", &configuration.namespace)?;
    validate_name("release", &configuration.release)?;
    if !(1..=3650).contains(&configuration.retention_days) {
        bail!("retention days must be between 1 and 3650");
    }

    let shared = aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(configuration.region.clone()))
        .load()
        .await;
    let sts = StsClient::new(&shared);
    let account = sts
        .get_caller_identity()
        .send()
        .await
        .context("cannot read the current AWS account")?
        .account()
        .context("AWS caller identity omitted its account")?
        .to_owned();
    let cluster = EksClient::new(&shared)
        .describe_cluster()
        .name(&configuration.cluster_name)
        .send()
        .await
        .context("cannot describe the EKS cluster")?
        .cluster
        .context("EKS omitted the cluster")?;
    let issuer = cluster
        .identity
        .and_then(|identity| identity.oidc)
        .and_then(|oidc| oidc.issuer)
        .context("the EKS cluster has no OIDC issuer")?;
    let issuer_name = issuer
        .strip_prefix("https://")
        .context("the EKS OIDC issuer is not HTTPS")?;

    let identity = resource_identity(configuration);
    let alias = format!("alias/devcenter/{identity}/vault-seal");
    let kms = KmsClient::new(&shared);
    let (key_id, key_arn) = ensure_kms_key(&kms, &alias, configuration).await?;

    let iam = IamClient::new(&shared);
    let vault_role_name = bounded_name(&format!("devcenter-{identity}-vault"), 64);
    let backup_role_name = bounded_name(&format!("devcenter-{identity}-vault-backup"), 64);
    let provider_arn = format!("arn:aws:iam::{account}:oidc-provider/{issuer_name}");
    let vault_subjects = [
        format!(
            "system:serviceaccount:{}:{}-vault",
            configuration.namespace, configuration.release
        ),
        format!(
            "system:serviceaccount:{}-vault-drill:{}-vault",
            configuration.namespace, configuration.release
        ),
    ];
    let backup_subject = format!(
        "system:serviceaccount:{}:{}-vault-backup",
        configuration.namespace, configuration.release
    );
    let vault_trust = trust_policy(&provider_arn, issuer_name, &vault_subjects);
    let backup_trust = trust_policy(&provider_arn, issuer_name, &[backup_subject]);
    let vault_role_arn = ensure_role(&iam, &vault_role_name, &vault_trust).await?;
    let backup_role_arn = ensure_role(&iam, &backup_role_name, &backup_trust).await?;
    iam.put_role_policy()
        .role_name(&vault_role_name)
        .policy_name("vault-seal")
        .policy_document(
            json!({
                "Version": "2012-10-17",
                "Statement": [{
                    "Effect": "Allow",
                    "Action": ["kms:Encrypt", "kms:Decrypt", "kms:DescribeKey"],
                    "Resource": key_arn
                }]
            })
            .to_string(),
        )
        .send()
        .await
        .context("cannot reconcile the Vault KMS policy")?;

    let bucket = bounded_bucket(&format!(
        "devcenter-vault-{}-{}-{}",
        account, configuration.region, identity
    ));
    let s3 = S3Client::new(&shared);
    ensure_bucket(
        &s3,
        &bucket,
        &configuration.region,
        &key_arn,
        configuration.retention_days,
    )
    .await?;
    iam.put_role_policy()
        .role_name(&backup_role_name)
        .policy_name("vault-backup")
        .policy_document(
            json!({
                "Version": "2012-10-17",
                "Statement": [
                    {
                        "Effect": "Allow",
                        "Action": ["s3:ListBucket"],
                        "Resource": format!("arn:aws:s3:::{bucket}")
                    },
                    {
                        "Effect": "Allow",
                        "Action": ["s3:GetObject", "s3:PutObject"],
                        "Resource": format!("arn:aws:s3:::{bucket}/snapshots/*")
                    },
                    {
                        "Effect": "Allow",
                        "Action": ["kms:Encrypt", "kms:Decrypt", "kms:GenerateDataKey", "kms:DescribeKey"],
                        "Resource": key_arn
                    }
                ]
            })
            .to_string(),
        )
        .send()
        .await
        .context("cannot reconcile the Vault backup policy")?;

    let output = Output {
        kms_key_arn: key_arn,
        vault_role_arn,
        backup_role_arn,
        backup_bucket: bucket,
        region: configuration.region.clone(),
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    let _ = key_id;
    Ok(())
}

async fn ensure_kms_key(
    kms: &KmsClient,
    alias: &str,
    configuration: &EnsureVault,
) -> Result<(String, String)> {
    let mut key_id = None;
    let mut pages = kms.list_aliases().into_paginator().send();
    while let Some(page) = pages.next().await {
        for candidate in page.context("cannot list KMS aliases")?.aliases() {
            if candidate.alias_name() == Some(alias) {
                key_id = candidate.target_key_id().map(ToOwned::to_owned);
            }
        }
    }
    let key_id = if let Some(key_id) = key_id {
        key_id
    } else {
        let created = kms
            .create_key()
            .description("Devcenter Vault auto-unseal and encrypted backups")
            .tags(
                Tag::builder()
                    .tag_key("devcenter:cluster")
                    .tag_value(&configuration.cluster_name)
                    .build()
                    .context("invalid KMS cluster tag")?,
            )
            .tags(
                Tag::builder()
                    .tag_key("devcenter:namespace")
                    .tag_value(&configuration.namespace)
                    .build()
                    .context("invalid KMS namespace tag")?,
            )
            .send()
            .await
            .context("cannot create the Vault KMS key")?;
        let key_id = created
            .key_metadata
            .map(|metadata| metadata.key_id)
            .context("KMS create-key omitted its identifier")?;
        kms.create_alias()
            .alias_name(alias)
            .target_key_id(&key_id)
            .send()
            .await
            .context("cannot create the Vault KMS alias")?;
        key_id
    };
    kms.enable_key_rotation()
        .key_id(&key_id)
        .send()
        .await
        .context("cannot enable Vault KMS key rotation")?;
    let metadata = kms
        .describe_key()
        .key_id(&key_id)
        .send()
        .await
        .context("cannot describe the Vault KMS key")?
        .key_metadata
        .context("KMS describe-key omitted metadata")?;
    let arn = metadata.arn.context("KMS key metadata omitted its ARN")?;
    Ok((key_id, arn))
}

async fn ensure_role(iam: &IamClient, name: &str, trust: &str) -> Result<String> {
    let role = match iam.get_role().role_name(name).send().await {
        Ok(output) => output.role,
        Err(error)
            if error.as_service_error().is_some_and(
                aws_sdk_iam::operation::get_role::GetRoleError::is_no_such_entity_exception,
            ) =>
        {
            iam.create_role()
                .role_name(name)
                .assume_role_policy_document(trust)
                .description("Devcenter Vault workload identity")
                .send()
                .await
                .context("cannot create a Vault IAM role")?
                .role
        }
        Err(error) => return Err(error).context("cannot inspect a Vault IAM role"),
    };
    let role = role.context("IAM omitted the Vault role")?;
    iam.update_assume_role_policy()
        .role_name(name)
        .policy_document(trust)
        .send()
        .await
        .context("cannot reconcile a Vault IAM trust policy")?;
    Ok(role.arn)
}

async fn ensure_bucket(
    s3: &S3Client,
    bucket: &str,
    region: &str,
    key_arn: &str,
    retention_days: i32,
) -> Result<()> {
    let exists = s3
        .list_buckets()
        .send()
        .await
        .context("cannot list S3 buckets")?
        .buckets()
        .iter()
        .any(|candidate| candidate.name() == Some(bucket));
    if !exists {
        let mut request = s3.create_bucket().bucket(bucket);
        if region != "us-east-1" {
            request = request.create_bucket_configuration(
                CreateBucketConfiguration::builder()
                    .location_constraint(BucketLocationConstraint::from(region))
                    .build(),
            );
        }
        request
            .send()
            .await
            .context("cannot create the Vault backup bucket")?;
    }
    s3.put_public_access_block()
        .bucket(bucket)
        .public_access_block_configuration(
            PublicAccessBlockConfiguration::builder()
                .block_public_acls(true)
                .ignore_public_acls(true)
                .block_public_policy(true)
                .restrict_public_buckets(true)
                .build(),
        )
        .send()
        .await
        .context("cannot block public access to the Vault backup bucket")?;
    s3.put_bucket_versioning()
        .bucket(bucket)
        .versioning_configuration(
            VersioningConfiguration::builder()
                .status(BucketVersioningStatus::Enabled)
                .build(),
        )
        .send()
        .await
        .context("cannot enable Vault backup bucket versioning")?;
    let encryption = ServerSideEncryptionByDefault::builder()
        .sse_algorithm(ServerSideEncryption::AwsKms)
        .kms_master_key_id(key_arn)
        .build()
        .context("invalid S3 encryption configuration")?;
    let rule = ServerSideEncryptionRule::builder()
        .apply_server_side_encryption_by_default(encryption)
        .bucket_key_enabled(true)
        .build();
    s3.put_bucket_encryption()
        .bucket(bucket)
        .server_side_encryption_configuration(
            ServerSideEncryptionConfiguration::builder()
                .rules(rule)
                .build()
                .context("invalid S3 bucket encryption configuration")?,
        )
        .send()
        .await
        .context("cannot enforce Vault backup bucket encryption")?;
    let lifecycle = LifecycleRule::builder()
        .id("expire-vault-snapshots")
        .status(aws_sdk_s3::types::ExpirationStatus::Enabled)
        .filter(LifecycleRuleFilter::builder().prefix("snapshots/").build())
        .expiration(LifecycleExpiration::builder().days(retention_days).build())
        .noncurrent_version_expiration(
            NoncurrentVersionExpiration::builder()
                .noncurrent_days(retention_days)
                .build(),
        )
        .build()
        .context("invalid Vault backup lifecycle rule")?;
    s3.put_bucket_lifecycle_configuration()
        .bucket(bucket)
        .lifecycle_configuration(
            BucketLifecycleConfiguration::builder()
                .rules(lifecycle)
                .build()
                .context("invalid Vault backup lifecycle configuration")?,
        )
        .send()
        .await
        .context("cannot reconcile Vault backup retention")?;
    Ok(())
}

fn trust_policy(provider_arn: &str, issuer: &str, subjects: &[String]) -> String {
    json!({
        "Version": "2012-10-17",
        "Statement": [{
            "Effect": "Allow",
            "Principal": {"Federated": provider_arn},
            "Action": "sts:AssumeRoleWithWebIdentity",
            "Condition": {"StringEquals": {
                format!("{issuer}:aud"): "sts.amazonaws.com",
                format!("{issuer}:sub"): subjects,
            }}
        }]
    })
    .to_string()
}

fn resource_identity(configuration: &EnsureVault) -> String {
    let source = format!(
        "{}/{}/{}",
        configuration.cluster_name, configuration.namespace, configuration.release
    );
    let suffix = format!("{:x}", Sha256::digest(source.as_bytes()));
    let prefix = bounded_name(
        &format!(
            "{}-{}-{}",
            configuration.cluster_name, configuration.namespace, configuration.release
        ),
        36,
    );
    format!("{prefix}-{}", &suffix[..12])
}

fn bounded_name(value: &str, maximum: usize) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .take(maximum)
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn bounded_bucket(value: &str) -> String {
    bounded_name(value, 63).replace('_', "-")
}

fn validate_name(label: &str, value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 128 || value.chars().any(char::is_whitespace) {
        bail!("{label} is empty, too long, or contains whitespace");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identities_are_stable_and_bounded() {
        let configuration = EnsureVault {
            cluster_name: "cluster-example".to_owned(),
            region: "eu-west-1".to_owned(),
            namespace: "devcenter".to_owned(),
            release: "devcenter".to_owned(),
            retention_days: 30,
        };
        assert_eq!(
            resource_identity(&configuration),
            resource_identity(&configuration)
        );
        assert!(resource_identity(&configuration).len() <= 49);
    }

    #[test]
    fn trust_is_bound_to_exact_service_account_subjects() {
        let policy = trust_policy(
            "arn:aws:iam::111111111111:oidc-provider/issuer.example",
            "issuer.example",
            &["system:serviceaccount:namespace:service".to_owned()],
        );
        assert!(policy.contains("system:serviceaccount:namespace:service"));
        assert!(!policy.contains("system:serviceaccount:namespace:*"));
    }
}
