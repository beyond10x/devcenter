//! Product composition of generated services into the hosted Devcenter Connector.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context as _, Result, bail};
use clap::Parser;
use connectors_runtime::{HostedRuntime, ServiceBundleBuilder};
use connectors_service::{
    DeploymentApproval, DeploymentRisk, OperationDeployment, ProviderIdentity, ServiceDeployment,
};
use serde::Deserialize;
use service_connectors::DurableEventStore;

const EVENTLOG_PREFIX: &str = "devcenter_services";

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Serve the Devcenter Connector with generated service modules"
)]
struct Args {
    /// Strict hosted Connectors configuration.
    #[arg(long)]
    config: PathBuf,
    /// Reviewed deployment overlays for generated services.
    #[arg(long)]
    service_deployments: PathBuf,
    /// Durable `PostgreSQL` Eventlog selected by the hosted deployment.
    #[arg(long, env = "DEVCENTER_CONNECTORS_EVENTLOG_DATABASE_URL")]
    eventlog_database_url: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DeploymentFile {
    services: Vec<Deployment>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Deployment {
    service_ref: String,
    provider: Provider,
    operations: BTreeMap<String, OperationPolicy>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Provider {
    #[serde(rename = "provider_ref")]
    reference: String,
    authority: String,
    connection_ref: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationPolicy {
    expose: bool,
    risk: Risk,
    approval: Approval,
    #[serde(default)]
    endpoint_bindings: BTreeMap<String, String>,
    #[serde(default)]
    credential_bindings: BTreeMap<String, String>,
    grant_refs: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Risk {
    Low,
    Medium,
    High,
    Destructive,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Approval {
    NotRequired,
    Required,
}

impl From<Deployment> for ServiceDeployment {
    fn from(deployment: Deployment) -> Self {
        Self {
            service_ref: deployment.service_ref,
            provider: ProviderIdentity {
                provider_ref: deployment.provider.reference,
                authority: deployment.provider.authority,
                connection_ref: deployment.provider.connection_ref,
            },
            operations: deployment
                .operations
                .into_iter()
                .map(|(operation, policy)| {
                    (
                        operation,
                        OperationDeployment {
                            expose: policy.expose,
                            risk: match policy.risk {
                                Risk::Low => DeploymentRisk::Low,
                                Risk::Medium => DeploymentRisk::Medium,
                                Risk::High => DeploymentRisk::High,
                                Risk::Destructive => DeploymentRisk::Destructive,
                            },
                            approval: match policy.approval {
                                Approval::NotRequired => DeploymentApproval::NotRequired,
                                Approval::Required => DeploymentApproval::Required,
                            },
                            endpoint_bindings: policy.endpoint_bindings,
                            credential_bindings: policy.credential_bindings,
                            grant_refs: policy.grant_refs,
                        },
                    )
                })
                .collect(),
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let store = Arc::new(
        eventlog_postgres::PostgresEventStore::connect(
            &args.eventlog_database_url,
            EVENTLOG_PREFIX,
        )
        .await
        .context("opening the generated-service PostgreSQL Eventlog")?,
    );
    let deployments = read_deployments(&args.service_deployments)?;
    let bundle = compose(store, deployments).await?;
    let runtime = HostedRuntime::bind_with_service_bundle(&args.config, bundle)
        .await
        .context("binding the composed hosted Connector")?;
    println!("{}", runtime.readiness());
    runtime
        .serve_until(shutdown())
        .await
        .context("serving the composed hosted Connector")?;
    Ok(())
}

fn read_deployments(path: &Path) -> Result<Vec<ServiceDeployment>> {
    let bytes = fs::read(path)
        .with_context(|| format!("reading service deployments from {}", path.display()))?;
    let parsed: DeploymentFile = serde_yaml::from_slice(&bytes)
        .with_context(|| format!("parsing service deployments from {}", path.display()))?;
    if parsed.services.is_empty() {
        bail!("the service deployment file must activate at least one service");
    }
    Ok(parsed.services.into_iter().map(Into::into).collect())
}

async fn compose(
    store: Arc<dyn DurableEventStore>,
    deployments: Vec<ServiceDeployment>,
) -> Result<connectors_runtime::ServiceBundle> {
    let mut builder = ServiceBundleBuilder::new();
    builder
        .register(todo_generated_service::connector_factory(store)?)
        .context("registering the generated Todo service")?;
    for deployment in deployments {
        builder
            .deploy(deployment)
            .context("admitting a generated-service deployment overlay")?;
    }
    builder
        .build()
        .await
        .context("binding the generated-service bundle")
}

async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;

    const OPERATIONS: &[&str] = &[
        "todo.add_item",
        "todo.archive_item",
        "todo.archive_list",
        "todo.complete_item",
        "todo.create_list",
        "todo.edit_item",
        "todo.expire_item",
        "todo.expire_list",
        "todo.get_item",
        "todo.get_list",
        "todo.list_items",
        "todo.list_visible_lists",
        "todo.rename_list",
        "todo.reopen_item",
        "todo.transfer_list",
    ];

    fn deployment_yaml() -> String {
        let mut operations = String::new();
        for operation in OPERATIONS {
            let approval =
                if operation.starts_with("todo.get_") || operation.starts_with("todo.list_") {
                    "not_required"
                } else {
                    "required"
                };
            let _ = write!(
                operations,
                "      {operation}:\n        expose: true\n        risk: low\n        approval: {approval}\n        grant_refs: [grant:todo:use]\n"
            );
        }
        format!(
            "services:\n  - service_ref: service:todo\n    provider:\n      provider_ref: provider:todo\n      authority: dev.b10x.todo\n      connection_ref: connection:todo\n    operations:\n{operations}"
        )
    }

    #[test]
    fn generated_todo_requires_an_exact_reviewed_overlay() {
        let parsed: DeploymentFile = serde_yaml::from_str(&deployment_yaml()).unwrap();
        assert_eq!(parsed.services.len(), 1);
        let deployment: ServiceDeployment = parsed.services.into_iter().next().unwrap().into();
        assert_eq!(deployment.service_ref, "service:todo");
        assert_eq!(
            deployment.operations.keys().cloned().collect::<Vec<_>>(),
            OPERATIONS
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
        assert!(deployment.operations.values().all(|policy| policy.expose));
    }

    #[test]
    fn deployment_file_has_no_authentication_coordinates() {
        let source = deployment_yaml();
        assert!(!source.contains("tenant"));
        assert!(!source.contains("realm"));
        assert!(!source.contains("user"));
    }
}
