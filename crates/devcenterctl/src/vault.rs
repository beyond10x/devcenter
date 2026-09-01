//! Vault lifecycle operations for the Devcenter deployment boundary.
//!
//! Secret-bearing values enter child processes only on standard input. They never implement
//! `Display` or `Debug`, and their backing bytes are zeroized on drop.

use anyhow::{Context, Result, bail};
use aws_config::BehaviorVersion;
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use reqwest::{Certificate, Client as HttpClient};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read as _, Write as _},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zeroize::{Zeroize, Zeroizing};

const TOKEN_SCRIPT: &str =
    "IFS= read -r token; export VAULT_TOKEN=\"$token\"; shift; exec vault \"$@\"";
const POLICY_SCRIPT: &str =
    "IFS= read -r token; export VAULT_TOKEN=\"$token\"; exec vault policy write \"$1\" -";
const KUBERNETES_HOST: &str = "https://kubernetes.default.svc:443";
const KUBERNETES_AUDIENCE: &str = "https://kubernetes.default.svc";

#[derive(Clone, Debug)]
pub struct Target {
    pub context: Option<String>,
    pub namespace: String,
    pub release: String,
}

impl Target {
    #[must_use]
    pub fn pod(&self) -> String {
        format!("{}-vault-0", self.release)
    }

    #[must_use]
    pub fn service_account(&self) -> String {
        format!("{}-vault", self.release)
    }
}

#[derive(Clone, Debug)]
pub struct Initialize {
    pub target: Target,
    pub tenant_id: String,
    pub mount: String,
    pub connectors_service_account: String,
    pub deployer_service_account: String,
    pub backup_service_account: String,
}

#[derive(Clone, Debug)]
pub struct Backup {
    pub address: String,
    pub ca_file: PathBuf,
    pub role: String,
    pub bucket: String,
    pub prefix: String,
    pub region: String,
    pub token_file: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Verify {
    pub initialization: Initialize,
}

#[derive(Clone, Debug)]
pub struct Migrate {
    pub source: Target,
    pub target: Target,
    pub source_mount: String,
    pub target_mount: String,
    pub tenant_id: String,
    pub deployer_service_account: String,
}

#[derive(Clone, Debug)]
pub struct RestoreDrill {
    pub target: Target,
    pub bucket: String,
    pub prefix: String,
    pub region: String,
    pub expected_mount: String,
    pub backup_service_account: String,
}

#[derive(Deserialize)]
struct InitResponse {
    recovery_keys_b64: Vec<String>,
    root_token: String,
}

#[derive(Deserialize)]
struct StatusResponse {
    initialized: bool,
    sealed: bool,
    storage_type: String,
}

#[derive(Serialize)]
struct KubernetesLogin<'a> {
    role: &'a str,
    jwt: &'a str,
}

pub fn initialize(configuration: &Initialize) -> Result<()> {
    validate_segment("tenant", &configuration.tenant_id)?;
    validate_segment("mount", &configuration.mount)?;
    keyring_probe(&configuration.target)?;

    let observed = status(&configuration.target)?;
    if observed.initialized {
        if observed.sealed {
            bail!("Vault is initialized but sealed; AWS KMS auto-unseal did not recover it");
        }
        reconcile_with_workload(configuration)?;
        println!("vault: already initialized; reconciled through workload identity");
        return Ok(());
    }

    let output = vault_without_token(
        &configuration.target,
        &[
            "operator",
            "init",
            "-format=json",
            "-recovery-shares=1",
            "-recovery-threshold=1",
        ],
        None,
    )?;
    let mut initialized: InitResponse = serde_json::from_slice(&output)
        .context("Vault returned invalid initialization material")?;
    if initialized.recovery_keys_b64.len() != 1 || initialized.root_token.is_empty() {
        initialized.root_token.zeroize();
        bail!("Vault initialization did not return one recovery share and a root token");
    }

    let recovery = Zeroizing::new(initialized.recovery_keys_b64.remove(0));
    let root = Zeroizing::new(std::mem::take(&mut initialized.root_token));
    keyring_store(&configuration.target, "recovery-key", recovery.as_bytes())?;
    let read_back = keyring_lookup(&configuration.target, "recovery-key")?
        .context("the recovery share disappeared from the operator keyring")?;
    if read_back.as_slice() != recovery.as_bytes() {
        bail!("the operator keyring did not return the recovery share byte-for-byte");
    }

    let reconciled = reconcile(configuration, root.as_str());
    let revoked = revoke(&configuration.target, root.as_str());
    match (reconciled, revoked) {
        (Err(error), Ok(())) => return Err(error),
        (Ok(()), Err(error)) => return Err(error).context("initial root-token revocation failed"),
        (Err(error), Err(_)) => {
            return Err(error)
                .context("reconciliation failed and root-token revocation also failed");
        }
        (Ok(()), Ok(())) => {}
    }

    let ready = status(&configuration.target)?;
    if !ready.initialized || ready.sealed || ready.storage_type != "raft" {
        bail!("Vault did not become initialized, unsealed, and Raft-backed");
    }
    println!("vault: initialized, reconciled, and initial root token revoked");
    Ok(())
}

pub async fn backup(configuration: &Backup) -> Result<()> {
    let pem = fs::read(&configuration.ca_file)
        .with_context(|| format!("cannot read {}", configuration.ca_file.display()))?;
    let certificate = Certificate::from_pem(&pem).context("Vault CA is not PEM")?;
    let client = HttpClient::builder()
        .add_root_certificate(certificate)
        .build()
        .context("cannot build Vault HTTP client")?;
    let jwt = Zeroizing::new(
        fs::read_to_string(&configuration.token_file)
            .with_context(|| format!("cannot read {}", configuration.token_file.display()))?,
    );
    let login = client
        .post(format!(
            "{}/v1/auth/kubernetes/login",
            configuration.address.trim_end_matches('/')
        ))
        .json(&KubernetesLogin {
            role: &configuration.role,
            jwt: jwt.trim(),
        })
        .send()
        .await
        .context("Vault Kubernetes login failed")?
        .error_for_status()
        .context("Vault refused the backup workload")?;
    let mut login: Value = login
        .json()
        .await
        .context("Vault login response is invalid")?;
    let token = login
        .pointer_mut("/auth/client_token")
        .and_then(Value::take_string)
        .map(Zeroizing::new)
        .context("Vault login omitted its client token")?;

    let result = backup_with_token(configuration, &client, token.as_str()).await;
    let revoked = client
        .post(format!(
            "{}/v1/auth/token/revoke-self",
            configuration.address.trim_end_matches('/')
        ))
        .header("X-Vault-Token", token.as_str())
        .send()
        .await
        .context("cannot revoke the backup Vault token")?;
    if !revoked.status().is_success() {
        bail!(
            "Vault backup token revocation failed with {}",
            revoked.status()
        );
    }
    result
}

pub fn verify(configuration: &Verify) -> Result<()> {
    let initialization = &configuration.initialization;
    let observed = status(&initialization.target)?;
    if !observed.initialized || observed.sealed || observed.storage_type != "raft" {
        bail!("Vault is not initialized, unsealed, and Raft-backed");
    }
    let token = workload_token(
        &initialization.target,
        &initialization.connectors_service_account,
        &format!("{}-connectors", initialization.target.release),
    )?;
    let allowed = format!(
        "{}/data/tenants/{}/com.devcenter.verify/sentinel",
        initialization.mount, initialization.tenant_id
    );
    let forbidden = format!(
        "{}/data/tenants/forbidden/com.devcenter.verify/sentinel",
        initialization.mount
    );
    let capabilities = String::from_utf8(vault_with_token(
        &initialization.target,
        token.as_str(),
        &["token", "capabilities", &allowed],
        None,
    )?)?;
    if !capabilities.contains("create") || !capabilities.contains("read") {
        bail!("Connectors workload lacks its required tenant capabilities");
    }
    let denied = String::from_utf8(vault_with_token(
        &initialization.target,
        token.as_str(),
        &["token", "capabilities", &forbidden],
        None,
    )?)?;
    if denied.trim() != "deny" {
        bail!("Connectors workload can reach a tenant outside its policy");
    }

    let sentinel = format!(
        "tenants/{}/com.devcenter.verify/sentinel",
        initialization.tenant_id
    );
    vault_with_token(
        &initialization.target,
        token.as_str(),
        &[
            "kv",
            "put",
            &format!("-mount={}", initialization.mount),
            &sentinel,
            "verified=true",
        ],
        None,
    )?;
    let value = String::from_utf8(vault_with_token(
        &initialization.target,
        token.as_str(),
        &[
            "kv",
            "get",
            "-field=verified",
            &format!("-mount={}", initialization.mount),
            &sentinel,
        ],
        None,
    )?)?;
    if value.trim() != "true" {
        bail!("Connectors workload did not read its sentinel byte-for-byte");
    }
    vault_with_token(
        &initialization.target,
        token.as_str(),
        &[
            "kv",
            "metadata",
            "delete",
            &format!("-mount={}", initialization.mount),
            &sentinel,
        ],
        None,
    )?;
    revoke(&initialization.target, token.as_str())?;
    println!("vault verification: exact tenant capabilities and destructive cleanup passed");
    Ok(())
}

pub fn migrate(configuration: &Migrate) -> Result<()> {
    validate_segment("source mount", &configuration.source_mount)?;
    validate_segment("target mount", &configuration.target_mount)?;
    validate_segment("tenant", &configuration.tenant_id)?;
    if configuration.source.context == configuration.target.context
        && configuration.source.namespace == configuration.target.namespace
        && configuration.source.release == configuration.target.release
    {
        bail!("source and target Vault resolve to the same deployment");
    }

    let mut source_bytes = Zeroizing::new(Vec::new());
    std::io::stdin()
        .read_to_end(&mut source_bytes)
        .context("cannot read the source Vault token from stdin")?;
    while source_bytes.last().is_some_and(u8::is_ascii_whitespace) {
        source_bytes.pop();
    }
    let source_token =
        std::str::from_utf8(&source_bytes).context("the source Vault token is not UTF-8")?;
    if source_token.is_empty() {
        bail!("source Vault token stdin is empty");
    }
    let target_token = workload_token(
        &configuration.target,
        &configuration.deployer_service_account,
        &format!("{}-vault-migration", configuration.target.release),
    )?;

    let root = format!("tenants/{}", configuration.tenant_id);
    let mut paths = Vec::new();
    list_secret_paths(
        &configuration.source,
        source_token,
        &configuration.source_mount,
        &root,
        &mut paths,
    )?;
    let migration = copy_records(configuration, source_token, target_token.as_str(), &paths);

    let revoked = revoke(&configuration.target, target_token.as_str());
    let finalized = finalize_migration(configuration);
    let (copied, identical) = match (migration, revoked, finalized) {
        (Ok(counts), Ok(()), Ok(())) => counts,
        (Err(error), Ok(()), Ok(())) => return Err(error),
        (Err(error), _, _) => return Err(error).context("migration cleanup also failed"),
        (Ok(_), Err(error), _) => return Err(error).context("migration token revocation failed"),
        (Ok(_), Ok(()), Err(error)) => return Err(error).context("migration role cleanup failed"),
    };
    println!(
        "vault migration: {} record(s), {copied} copied, {identical} already identical",
        paths.len()
    );
    Ok(())
}

fn copy_records(
    configuration: &Migrate,
    source_token: &str,
    target_token: &str,
    paths: &[String],
) -> Result<(usize, usize)> {
    let mut copied = 0_usize;
    let mut identical = 0_usize;
    for path in paths {
        let source_path = format!("{}/data/{path}", configuration.source_mount);
        let source_data = Zeroizing::new(vault_with_token(
            &configuration.source,
            source_token,
            &["read", "-format=json", "-field=data", &source_path],
            None,
        )?);
        let target_path = format!("{}/data/{path}", configuration.target_mount);
        let target = vault_with_token_output(
            &configuration.target,
            target_token,
            &["read", "-format=json", "-field=data", &target_path],
            None,
        )?;
        if target.status.success() {
            if Sha256::digest(&target.stdout) == Sha256::digest(source_data.as_slice()) {
                migrate_secret_metadata(configuration, source_token, target_token, path)?;
                identical += 1;
                continue;
            }
            bail!("the migration target contains a conflicting credential record");
        }

        let mut payload = Zeroizing::new(Vec::with_capacity(source_data.len() + 10));
        payload.extend_from_slice(b"{\"data\":");
        payload.extend_from_slice(source_data.as_slice());
        payload.push(b'}');
        vault_with_token(
            &configuration.target,
            target_token,
            &["write", &target_path, "-"],
            Some(payload.as_slice()),
        )?;
        let read_back = Zeroizing::new(vault_with_token(
            &configuration.target,
            target_token,
            &["read", "-format=json", "-field=data", &target_path],
            None,
        )?);
        if Sha256::digest(read_back.as_slice()) != Sha256::digest(source_data.as_slice()) {
            bail!("a migrated credential did not read back byte-for-byte");
        }
        migrate_secret_metadata(configuration, source_token, target_token, path)?;
        copied += 1;
    }
    Ok((copied, identical))
}

fn migrate_secret_metadata(
    configuration: &Migrate,
    source_token: &str,
    target_token: &str,
    path: &str,
) -> Result<()> {
    let source_path = format!("{}/metadata/{path}", configuration.source_mount);
    let source = Zeroizing::new(vault_with_token(
        &configuration.source,
        source_token,
        &["read", "-format=json", "-field=data", &source_path],
        None,
    )?);
    let source: Value = serde_json::from_slice(source.as_slice())
        .context("source Vault returned invalid secret metadata")?;
    let mut mutable = serde_json::Map::new();
    for field in [
        "cas_required",
        "custom_metadata",
        "delete_version_after",
        "max_versions",
    ] {
        if let Some(value) = source.get(field) {
            mutable.insert(field.to_owned(), value.clone());
        }
    }
    let payload = Zeroizing::new(serde_json::to_vec(&Value::Object(mutable))?);
    let target_path = format!("{}/metadata/{path}", configuration.target_mount);
    vault_with_token(
        &configuration.target,
        target_token,
        &["write", &target_path, "-"],
        Some(payload.as_slice()),
    )?;
    Ok(())
}

pub async fn restore_drill(configuration: &RestoreDrill) -> Result<()> {
    let observed = status(&configuration.target)?;
    if observed.initialized {
        bail!("restore drill target must be a fresh, uninitialized Vault");
    }
    let output = vault_without_token(
        &configuration.target,
        &[
            "operator",
            "init",
            "-format=json",
            "-recovery-shares=1",
            "-recovery-threshold=1",
        ],
        None,
    )?;
    let mut initialized: InitResponse = serde_json::from_slice(&output)?;
    let root = Zeroizing::new(std::mem::take(&mut initialized.root_token));
    for key in &mut initialized.recovery_keys_b64 {
        key.zeroize();
    }

    let sdk = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(configuration.region.clone()))
        .load()
        .await;
    let s3 = S3Client::new(&sdk);
    let listing = s3
        .list_objects_v2()
        .bucket(&configuration.bucket)
        .prefix(configuration.prefix.trim_matches('/'))
        .send()
        .await
        .context("cannot list Vault snapshots")?;
    let key = listing
        .contents()
        .iter()
        .filter_map(|object| object.key())
        .max()
        .context("the Vault backup bucket contains no snapshots")?
        .to_owned();
    let snapshot = s3
        .get_object()
        .bucket(&configuration.bucket)
        .key(&key)
        .send()
        .await
        .context("cannot download the Vault restore snapshot")?
        .body
        .collect()
        .await
        .context("cannot read the Vault restore snapshot")?
        .into_bytes();
    vault_with_token(
        &configuration.target,
        root.as_str(),
        &["operator", "raft", "snapshot", "restore", "-force", "-"],
        Some(&snapshot),
    )?;

    let mut ready = false;
    for _ in 0..30 {
        thread::sleep(Duration::from_secs(2));
        if status(&configuration.target).is_ok_and(|status| {
            status.initialized && !status.sealed && status.storage_type == "raft"
        }) {
            ready = true;
            break;
        }
    }
    if !ready {
        bail!("restored Vault did not auto-unseal within 60 seconds");
    }
    let token = workload_token(
        &configuration.target,
        &configuration.backup_service_account,
        &format!("{}-vault-backup", configuration.target.release),
    )?;
    let mounts: Value = serde_json::from_slice(&vault_with_token(
        &configuration.target,
        token.as_str(),
        &["secrets", "list", "-format=json"],
        None,
    )?)?;
    if mounts
        .get(format!("{}/", configuration.expected_mount))
        .is_none()
    {
        bail!("restored Vault does not contain the expected credential mount");
    }
    revoke(&configuration.target, token.as_str())?;
    println!("vault restore drill: restored and read {key}");
    Ok(())
}

async fn backup_with_token(configuration: &Backup, client: &HttpClient, token: &str) -> Result<()> {
    let response = client
        .get(format!(
            "{}/v1/sys/storage/raft/snapshot",
            configuration.address.trim_end_matches('/')
        ))
        .header("X-Vault-Token", token)
        .send()
        .await
        .context("Vault snapshot request failed")?
        .error_for_status()
        .context("Vault refused the Raft snapshot")?;
    let snapshot = response
        .bytes()
        .await
        .context("cannot read Vault snapshot")?;
    let digest = format!("{:x}", Sha256::digest(&snapshot));
    let epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock predates the Unix epoch")?
        .as_secs();
    let key = format!("{}/{epoch}.snap", configuration.prefix.trim_matches('/'));
    let sdk = aws_config::defaults(BehaviorVersion::latest())
        .region(aws_config::Region::new(configuration.region.clone()))
        .load()
        .await;
    S3Client::new(&sdk)
        .put_object()
        .bucket(&configuration.bucket)
        .key(&key)
        .metadata("sha256", &digest)
        .body(ByteStream::from(snapshot))
        .send()
        .await
        .context("cannot upload Vault snapshot")?;
    println!("vault backup: uploaded {key} sha256:{digest}");
    Ok(())
}

fn reconcile_with_workload(configuration: &Initialize) -> Result<()> {
    let jwt = service_account_token(
        &configuration.target,
        &configuration.deployer_service_account,
    )?;
    let output = vault_without_token(
        &configuration.target,
        &[
            "write",
            "-format=json",
            "auth/kubernetes/login",
            &format!("role={}-vault-operator", configuration.target.release),
            "jwt=-",
        ],
        Some(jwt.as_slice()),
    )?;
    let mut response: Value =
        serde_json::from_slice(&output).context("Vault workload login response is invalid")?;
    let token = response
        .pointer_mut("/auth/client_token")
        .and_then(Value::take_string)
        .map(Zeroizing::new)
        .context("Vault workload login omitted a token")?;
    reconcile(configuration, token.as_str())
}

fn reconcile(configuration: &Initialize, token: &str) -> Result<()> {
    let mounts: Value = serde_json::from_slice(&vault_with_token(
        &configuration.target,
        token,
        &["secrets", "list", "-format=json"],
        None,
    )?)?;
    let mount_key = format!("{}/", configuration.mount);
    if mounts.get(&mount_key).is_none() {
        vault_with_token(
            &configuration.target,
            token,
            &[
                "secrets",
                "enable",
                &format!("-path={}", configuration.mount),
                "kv-v2",
            ],
            None,
        )?;
    }

    let auth: Value = serde_json::from_slice(&vault_with_token(
        &configuration.target,
        token,
        &["auth", "list", "-format=json"],
        None,
    )?)?;
    if auth.get("kubernetes/").is_none() {
        vault_with_token(
            &configuration.target,
            token,
            &["auth", "enable", "kubernetes"],
            None,
        )?;
    }
    vault_with_token(
        &configuration.target,
        token,
        &[
            "write",
            "auth/kubernetes/config",
            &format!("kubernetes_host={KUBERNETES_HOST}"),
        ],
        None,
    )?;

    let connectors_policy = format!(
        "path \"{0}/data/tenants/{1}/*\" {{ capabilities = [\"create\", \"update\", \"read\", \"delete\"] }}\n\
         path \"{0}/metadata/tenants/{1}\" {{ capabilities = [\"read\", \"list\"] }}\n\
         path \"{0}/metadata/tenants/{1}/*\" {{ capabilities = [\"read\", \"list\", \"delete\"] }}\n",
        configuration.mount, configuration.tenant_id
    );
    write_policy(
        &configuration.target,
        token,
        &format!("{}-connectors", configuration.target.release),
        &connectors_policy,
    )?;
    write_policy(
        &configuration.target,
        token,
        &format!("{}-vault-backup", configuration.target.release),
        "path \"sys/storage/raft/snapshot\" { capabilities = [\"read\"] }\n\
         path \"sys/mounts\" { capabilities = [\"read\"] }\n",
    )?;
    let operator_policy = format!(
        "path \"sys/mounts\" {{ capabilities = [\"read\"] }}\n\
         path \"sys/auth\" {{ capabilities = [\"read\"] }}\n\
         path \"sys/mounts/{0}\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"sys/mounts/{0}/*\" {{ capabilities = [\"create\", \"read\", \"update\", \"delete\"] }}\n\
         path \"sys/policies/acl/{1}-connectors\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"sys/policies/acl/{1}-vault-backup\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"sys/policies/acl/{1}-vault-operator\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"sys/policies/acl/{1}-vault-migration\" {{ capabilities = [\"create\", \"read\", \"update\", \"delete\"] }}\n\
         path \"auth/kubernetes/config\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"auth/kubernetes/role/{1}-connectors\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"auth/kubernetes/role/{1}-vault-backup\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"auth/kubernetes/role/{1}-vault-operator\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"auth/kubernetes/role/{1}-vault-migration\" {{ capabilities = [\"create\", \"read\", \"update\", \"delete\"] }}\n\
         path \"sys/audit\" {{ capabilities = [\"read\"] }}\n\
         path \"sys/audit/file\" {{ capabilities = [\"create\", \"read\", \"update\"] }}\n\
         path \"sys/health\" {{ capabilities = [\"read\"] }}\n",
        configuration.mount, configuration.target.release
    );
    write_policy(
        &configuration.target,
        token,
        &format!("{}-vault-operator", configuration.target.release),
        &operator_policy,
    )?;

    write_kubernetes_role(
        configuration,
        token,
        &KubernetesRole {
            role: &format!("{}-connectors", configuration.target.release),
            service_account: &configuration.connectors_service_account,
            policy: &format!("{}-connectors", configuration.target.release),
            namespaces: &configuration.target.namespace,
            ttl: "15m",
            max_ttl: "1h",
        },
    )?;
    write_kubernetes_role(
        configuration,
        token,
        &KubernetesRole {
            role: &format!("{}-vault-backup", configuration.target.release),
            service_account: &configuration.backup_service_account,
            policy: &format!("{}-vault-backup", configuration.target.release),
            namespaces: &format!(
                "{},{}-vault-drill",
                configuration.target.namespace, configuration.target.namespace
            ),
            ttl: "15m",
            max_ttl: "15m",
        },
    )?;
    write_kubernetes_role(
        configuration,
        token,
        &KubernetesRole {
            role: &format!("{}-vault-operator", configuration.target.release),
            service_account: &configuration.deployer_service_account,
            policy: &format!("{}-vault-operator", configuration.target.release),
            namespaces: &configuration.target.namespace,
            ttl: "10m",
            max_ttl: "10m",
        },
    )?;

    let migration_policy = format!(
        "path \"{0}/data/tenants/{1}/*\" {{ capabilities = [\"create\", \"update\", \"read\", \"delete\"] }}\n\
         path \"{0}/metadata/tenants/{1}\" {{ capabilities = [\"read\", \"list\"] }}\n\
         path \"{0}/metadata/tenants/{1}/*\" {{ capabilities = [\"create\", \"update\", \"read\", \"list\", \"delete\"] }}\n",
        configuration.mount, configuration.tenant_id
    );
    write_policy(
        &configuration.target,
        token,
        &format!("{}-vault-migration", configuration.target.release),
        &migration_policy,
    )?;
    write_kubernetes_role(
        configuration,
        token,
        &KubernetesRole {
            role: &format!("{}-vault-migration", configuration.target.release),
            service_account: &configuration.deployer_service_account,
            policy: &format!("{}-vault-migration", configuration.target.release),
            namespaces: &configuration.target.namespace,
            ttl: "10m",
            max_ttl: "10m",
        },
    )?;

    let audits: Value = serde_json::from_slice(&vault_with_token(
        &configuration.target,
        token,
        &["audit", "list", "-format=json"],
        None,
    )?)?;
    if audits.get("file/").is_none() {
        vault_with_token(
            &configuration.target,
            token,
            &[
                "audit",
                "enable",
                "file",
                "file_path=/vault/audit/audit.log",
            ],
            None,
        )?;
    }
    Ok(())
}

struct KubernetesRole<'a> {
    role: &'a str,
    service_account: &'a str,
    policy: &'a str,
    namespaces: &'a str,
    ttl: &'a str,
    max_ttl: &'a str,
}

fn write_kubernetes_role(
    configuration: &Initialize,
    token: &str,
    role: &KubernetesRole<'_>,
) -> Result<()> {
    vault_with_token(
        &configuration.target,
        token,
        &[
            "write",
            &format!("auth/kubernetes/role/{}", role.role),
            &format!("bound_service_account_names={}", role.service_account),
            &format!("bound_service_account_namespaces={}", role.namespaces),
            &format!("audience={KUBERNETES_AUDIENCE}"),
            &format!("token_policies={}", role.policy),
            &format!("token_ttl={}", role.ttl),
            &format!("token_max_ttl={}", role.max_ttl),
        ],
        None,
    )?;
    Ok(())
}

fn write_policy(target: &Target, token: &str, name: &str, policy: &str) -> Result<()> {
    let mut input = Zeroizing::new(Vec::with_capacity(token.len() + policy.len() + 2));
    input.extend_from_slice(token.as_bytes());
    input.push(b'\n');
    input.extend_from_slice(policy.as_bytes());
    kubectl_exec(
        target,
        &["/bin/sh", "-ec", POLICY_SCRIPT, "--", name],
        Some(&input),
    )?;
    Ok(())
}

fn workload_token(target: &Target, service_account: &str, role: &str) -> Result<Zeroizing<String>> {
    let jwt = service_account_token(target, service_account)?;
    let output = vault_without_token(
        target,
        &[
            "write",
            "-format=json",
            "auth/kubernetes/login",
            &format!("role={role}"),
            "jwt=-",
        ],
        Some(jwt.as_slice()),
    )?;
    let mut response: Value =
        serde_json::from_slice(&output).context("Vault workload login response is invalid")?;
    response
        .pointer_mut("/auth/client_token")
        .and_then(Value::take_string)
        .map(Zeroizing::new)
        .context("Vault workload login omitted a token")
}

fn revoke(target: &Target, token: &str) -> Result<()> {
    vault_with_token(target, token, &["token", "revoke", "-self"], None)?;
    Ok(())
}

fn list_secret_paths(
    target: &Target,
    token: &str,
    mount: &str,
    prefix: &str,
    paths: &mut Vec<String>,
) -> Result<()> {
    let endpoint = format!("{mount}/metadata/{prefix}");
    let response: Value = serde_json::from_slice(&vault_with_token(
        target,
        token,
        &["list", "-format=json", &endpoint],
        None,
    )?)?;
    let keys = response
        .pointer("/data/keys")
        .and_then(Value::as_array)
        .context("Vault metadata listing omitted its keys")?;
    for key in keys {
        let key = key
            .as_str()
            .context("Vault returned a non-string metadata key")?;
        if key.ends_with('/') {
            list_secret_paths(
                target,
                token,
                mount,
                &format!("{prefix}/{}", key.trim_end_matches('/')),
                paths,
            )?;
        } else {
            paths.push(format!("{prefix}/{key}"));
        }
    }
    Ok(())
}

fn finalize_migration(configuration: &Migrate) -> Result<()> {
    let token = workload_token(
        &configuration.target,
        &configuration.deployer_service_account,
        &format!("{}-vault-operator", configuration.target.release),
    )?;
    vault_with_token(
        &configuration.target,
        token.as_str(),
        &[
            "delete",
            &format!(
                "auth/kubernetes/role/{}-vault-migration",
                configuration.target.release
            ),
        ],
        None,
    )?;
    vault_with_token(
        &configuration.target,
        token.as_str(),
        &[
            "policy",
            "delete",
            &format!("{}-vault-migration", configuration.target.release),
        ],
        None,
    )?;
    revoke(&configuration.target, token.as_str())
}

fn status(target: &Target) -> Result<StatusResponse> {
    let output = vault_without_token_allow_failure(target, &["status", "-format=json"], None)?;
    serde_json::from_slice(&output).context("Vault status response is invalid")
}

fn service_account_token(target: &Target, service_account: &str) -> Result<Zeroizing<Vec<u8>>> {
    let mut command = kubectl(target);
    command.args([
        "-n",
        &target.namespace,
        "create",
        "token",
        service_account,
        &format!("--audience={KUBERNETES_AUDIENCE}"),
        "--duration=10m",
    ]);
    let output = run_capture(&mut command, None)?;
    Ok(Zeroizing::new(output))
}

fn vault_with_token(
    target: &Target,
    token: &str,
    args: &[&str],
    payload: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut input = Zeroizing::new(Vec::with_capacity(
        token.len() + payload.map_or(0, <[u8]>::len) + 1,
    ));
    input.extend_from_slice(token.as_bytes());
    input.push(b'\n');
    if let Some(payload) = payload {
        input.extend_from_slice(payload);
    }
    let mut command_args = vec!["/bin/sh", "-ec", TOKEN_SCRIPT, "--", "ignored"];
    command_args.extend_from_slice(args);
    kubectl_exec(target, &command_args, Some(&input))
}

fn vault_with_token_output(
    target: &Target,
    token: &str,
    args: &[&str],
    payload: Option<&[u8]>,
) -> Result<std::process::Output> {
    let mut input = Zeroizing::new(Vec::with_capacity(
        token.len() + payload.map_or(0, <[u8]>::len) + 1,
    ));
    input.extend_from_slice(token.as_bytes());
    input.push(b'\n');
    if let Some(payload) = payload {
        input.extend_from_slice(payload);
    }
    let mut command_args = vec!["/bin/sh", "-ec", TOKEN_SCRIPT, "--", "ignored"];
    command_args.extend_from_slice(args);
    let mut command = kubectl(target);
    command.args(["-n", &target.namespace, "exec", "-i", &target.pod(), "--"]);
    command.args(&command_args);
    run(&mut command, Some(&input))
}

fn vault_without_token(target: &Target, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut command_args = vec!["vault"];
    command_args.extend_from_slice(args);
    kubectl_exec(target, &command_args, input)
}

fn vault_without_token_allow_failure(
    target: &Target,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut command_args = vec!["vault"];
    command_args.extend_from_slice(args);
    kubectl_exec_allow_failure(target, &command_args, input)
}

fn kubectl_exec(target: &Target, args: &[&str], input: Option<&[u8]>) -> Result<Vec<u8>> {
    let mut command = kubectl(target);
    command.args(["-n", &target.namespace, "exec", "-i", &target.pod(), "--"]);
    command.args(args);
    run_capture(&mut command, input)
}

fn kubectl_exec_allow_failure(
    target: &Target,
    args: &[&str],
    input: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let mut command = kubectl(target);
    command.args(["-n", &target.namespace, "exec", "-i", &target.pod(), "--"]);
    command.args(args);
    let output = run(&mut command, input)?;
    if output.stdout.is_empty() {
        bail!("kubectl exec returned no Vault response");
    }
    Ok(output.stdout)
}

fn kubectl(target: &Target) -> Command {
    let mut command = Command::new("kubectl");
    if let Some(context) = &target.context {
        command.args(["--context", context]);
    }
    command
}

fn run_capture(command: &mut Command, input: Option<&[u8]>) -> Result<Vec<u8>> {
    let output = run(command, input)?;
    if !output.status.success() {
        bail!(
            "{} exited with {}",
            command.get_program().to_string_lossy(),
            output.status
        );
    }
    Ok(output.stdout)
}

fn run(command: &mut Command, input: Option<&[u8]>) -> Result<std::process::Output> {
    command.stdout(Stdio::piped()).stderr(Stdio::inherit());
    if input.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .with_context(|| format!("cannot run {}", command.get_program().to_string_lossy()))?;
    if let Some(input) = input {
        child
            .stdin
            .take()
            .context("child stdin is unavailable")?
            .write_all(input)
            .context("cannot write child stdin")?;
    }
    child
        .wait_with_output()
        .context("cannot collect child output")
}

fn keyring_probe(target: &Target) -> Result<()> {
    let mut random = [0_u8; 32];
    getrandom::fill(&mut random)
        .map_err(|error| anyhow::anyhow!("cannot create keyring probe: {error}"))?;
    let material = Zeroizing::new(URL_SAFE_NO_PAD.encode(random));
    random.zeroize();
    let nonce = &material[..16];
    let name = format!("preflight-{nonce}");
    keyring_store(target, &name, material.as_bytes())?;
    let read_back = keyring_lookup(target, &name)?.context("keyring probe disappeared")?;
    if read_back.as_slice() != material.as_bytes() {
        bail!("operator keyring probe did not round-trip byte-for-byte");
    }
    keyring_clear(target, &name)?;
    Ok(())
}

fn keyring_store(target: &Target, material: &str, secret: &[u8]) -> Result<()> {
    let mut command = Command::new("secret-tool");
    command.args(["store", "--label", "Devcenter Vault recovery material"]);
    keyring_attributes(&mut command, target, material);
    run_capture(&mut command, Some(secret))?;
    Ok(())
}

fn keyring_lookup(target: &Target, material: &str) -> Result<Option<Zeroizing<Vec<u8>>>> {
    let mut command = Command::new("secret-tool");
    command.arg("lookup");
    keyring_attributes(&mut command, target, material);
    let output = run(&mut command, None)?;
    if output.status.success() {
        let mut bytes = Zeroizing::new(output.stdout);
        while bytes.last().is_some_and(u8::is_ascii_whitespace) {
            bytes.pop();
        }
        Ok(Some(bytes))
    } else {
        Ok(None)
    }
}

fn keyring_clear(target: &Target, material: &str) -> Result<()> {
    let mut command = Command::new("secret-tool");
    command.arg("clear");
    keyring_attributes(&mut command, target, material);
    run_capture(&mut command, None)?;
    Ok(())
}

fn keyring_attributes(command: &mut Command, target: &Target, material: &str) {
    command.args([
        "application",
        "devcenter-vault",
        "kube-context",
        target.context.as_deref().unwrap_or("current"),
        "namespace",
        &target.namespace,
        "release",
        &target.release,
        "material",
        material,
    ]);
}

fn validate_segment(label: &str, value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || value.contains('/')
        || value.chars().any(char::is_whitespace)
    {
        bail!("{label} is not a safe Vault path segment");
    }
    Ok(())
}

trait TakeString {
    fn take_string(&mut self) -> Option<String>;
}

impl TakeString for Value {
    fn take_string(&mut self) -> Option<String> {
        match self.take() {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_segments_cannot_escape_their_mount() {
        assert!(validate_segment("tenant", "tenant-example").is_ok());
        assert!(validate_segment("tenant", "../other").is_err());
        assert!(validate_segment("tenant", "tenant other").is_err());
    }

    #[test]
    fn snapshot_names_carry_no_tenant_or_provider() {
        let prefix = "snapshots";
        let key = format!("{prefix}/{}.snap", 1_700_000_000_u64);
        assert_eq!(key, "snapshots/1700000000.snap");
    }
}
