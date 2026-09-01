use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use devcenterctl::vault;
use devcenterctl::{cloud, leak};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[derive(Debug, Parser)]
#[command(version, about = "Verify, deploy, inspect, and roll back Devcenter")]
struct Cli {
    #[command(subcommand)]
    command: Action,
}

#[derive(Debug, Subcommand)]
enum Action {
    LeakCheck(LeakCheck),
    Render(HelmTarget),
    Preflight(ClusterTarget),
    Apply(HelmTarget),
    Status(ReleaseTarget),
    Verify(Verify),
    Rollback(Rollback),
    Mirror(Mirror),
    #[command(subcommand)]
    Release(ReleaseAction),
    #[command(subcommand)]
    Bundle(BundleAction),
    #[command(subcommand)]
    Vault(VaultAction),
    #[command(subcommand)]
    Infrastructure(InfrastructureAction),
}

#[derive(Debug, Subcommand)]
enum InfrastructureAction {
    #[command(subcommand)]
    Aws(AwsAction),
}

#[derive(Debug, Subcommand)]
enum AwsAction {
    EnsureVault(AwsEnsureVault),
}

#[derive(Debug, Args)]
struct AwsEnsureVault {
    #[arg(long)]
    cluster_name: String,
    #[arg(long)]
    region: String,
    #[arg(long)]
    namespace: String,
    #[arg(long, default_value = "devcenter")]
    release: String,
    #[arg(long, default_value_t = 30)]
    retention_days: i32,
}

#[derive(Debug, Subcommand)]
enum VaultAction {
    Initialize(VaultInitialize),
    Backup(VaultBackup),
    Verify(VaultVerify),
    MigrateKv(VaultMigrate),
    RestoreDrill(VaultRestoreDrill),
}

#[derive(Debug, Args)]
struct VaultInitialize {
    #[command(flatten)]
    target: ReleaseTarget,
    #[arg(long)]
    tenant_id: String,
    #[arg(long, default_value = "connectors")]
    mount: String,
    #[arg(long)]
    connectors_service_account: String,
    #[arg(long)]
    deployer_service_account: String,
    #[arg(long)]
    backup_service_account: String,
}

#[derive(Debug, Args)]
struct VaultBackup {
    #[arg(long)]
    address: String,
    #[arg(long)]
    ca_file: PathBuf,
    #[arg(long)]
    role: String,
    #[arg(long)]
    bucket: String,
    #[arg(long, default_value = "snapshots")]
    prefix: String,
    #[arg(long)]
    region: String,
    #[arg(
        long,
        default_value = "/var/run/secrets/kubernetes.io/serviceaccount/token"
    )]
    token_file: PathBuf,
}

#[derive(Debug, Args)]
struct VaultVerify {
    #[command(flatten)]
    initialize: VaultInitialize,
}

#[derive(Debug, Args)]
struct VaultMigrate {
    #[arg(long)]
    source_context: Option<String>,
    #[arg(long)]
    source_namespace: String,
    #[arg(long)]
    source_release: String,
    #[arg(long)]
    target_context: Option<String>,
    #[arg(long)]
    target_namespace: String,
    #[arg(long, default_value = "devcenter")]
    target_release: String,
    #[arg(long)]
    tenant_id: String,
    #[arg(long, default_value = "connectors")]
    source_mount: String,
    #[arg(long, default_value = "connectors")]
    target_mount: String,
    #[arg(long)]
    deployer_service_account: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Debug, Args)]
struct VaultRestoreDrill {
    #[command(flatten)]
    target: ReleaseTarget,
    #[arg(long)]
    bucket: String,
    #[arg(long, default_value = "snapshots")]
    prefix: String,
    #[arg(long)]
    region: String,
    #[arg(long, default_value = "connectors")]
    expected_mount: String,
    #[arg(long)]
    backup_service_account: String,
    #[arg(long)]
    yes: bool,
}

#[derive(Debug, Args)]
struct LeakCheck {
    #[arg(long)]
    root: PathBuf,
    #[arg(long)]
    deny_file: PathBuf,
}

#[derive(Debug, Clone, Args)]
struct ReleaseTarget {
    #[arg(long, default_value = "devcenter")]
    release: String,
    #[arg(long)]
    namespace: String,
    #[arg(long)]
    context: Option<String>,
}

#[derive(Debug, Args)]
struct ClusterTarget {
    #[command(flatten)]
    target: ReleaseTarget,
    #[arg(long, default_value = "amd64")]
    required_architecture: String,
}

#[derive(Debug, Clone, Args)]
struct HelmTarget {
    #[command(flatten)]
    target: ReleaseTarget,
    #[arg(long)]
    chart: String,
    #[arg(long)]
    version: Option<String>,
    #[arg(long)]
    values: PathBuf,
    #[arg(long, default_value = "10m")]
    timeout: String,
    #[arg(long)]
    create_namespace: bool,
    #[arg(long)]
    initialize_vault: bool,
    #[arg(long)]
    vault_tenant_id: Option<String>,
    #[arg(long, default_value = "connectors")]
    vault_mount: String,
    #[arg(long)]
    vault_connectors_service_account: Option<String>,
    #[arg(long)]
    vault_deployer_service_account: Option<String>,
    #[arg(long)]
    vault_backup_service_account: Option<String>,
}

#[derive(Debug, Args)]
struct Verify {
    #[arg(long)]
    origin: String,
}

#[derive(Debug, Args)]
struct Rollback {
    #[command(flatten)]
    target: ReleaseTarget,
    #[arg(long)]
    revision: u32,
    #[arg(long, default_value = "10m")]
    timeout: String,
    #[arg(long)]
    yes: bool,
    #[arg(long)]
    allow_secret_store_removal: bool,
}

#[derive(Debug, Args)]
struct Mirror {
    #[arg(long)]
    source: String,
    #[arg(long)]
    destination: String,
}

#[derive(Debug, Subcommand)]
enum ReleaseAction {
    Verify(ReleaseVerify),
}

#[derive(Debug, Args)]
struct ReleaseVerify {
    #[arg(long)]
    artifact: String,
    #[arg(long)]
    expected_digest: String,
}

#[derive(Debug, Subcommand)]
enum BundleAction {
    Validate(BundleValidate),
}

#[derive(Debug, Args)]
struct BundleValidate {
    #[arg(long)]
    root: PathBuf,
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().command {
        Action::LeakCheck(args) => leak_check(&args),
        Action::Render(args) => render(&args),
        Action::Preflight(args) => preflight(&args),
        Action::Apply(args) => apply(&args),
        Action::Status(args) => status(&args),
        Action::Verify(args) => verify(&args),
        Action::Rollback(args) => rollback(&args),
        Action::Mirror(args) => mirror(&args),
        Action::Release(ReleaseAction::Verify(args)) => release_verify(&args),
        Action::Bundle(BundleAction::Validate(args)) => bundle_validate(&args),
        Action::Vault(VaultAction::Initialize(args)) => vault::initialize(&vault::Initialize {
            target: vault::Target {
                context: args.target.context,
                namespace: args.target.namespace,
                release: args.target.release,
            },
            tenant_id: args.tenant_id,
            mount: args.mount,
            connectors_service_account: args.connectors_service_account,
            deployer_service_account: args.deployer_service_account,
            backup_service_account: args.backup_service_account,
        }),
        Action::Vault(VaultAction::Backup(args)) => {
            vault::backup(&vault::Backup {
                address: args.address,
                ca_file: args.ca_file,
                role: args.role,
                bucket: args.bucket,
                prefix: args.prefix,
                region: args.region,
                token_file: args.token_file,
            })
            .await
        }
        Action::Vault(VaultAction::Verify(args)) => vault::verify(&vault::Verify {
            initialization: vault_initialize(args.initialize),
        }),
        Action::Vault(VaultAction::MigrateKv(args)) => {
            if !args.yes {
                bail!("vault migrate-kv requires --yes and an explicit source and target");
            }
            vault::migrate(&vault::Migrate {
                source: vault::Target {
                    context: args.source_context,
                    namespace: args.source_namespace,
                    release: args.source_release,
                },
                target: vault::Target {
                    context: args.target_context,
                    namespace: args.target_namespace,
                    release: args.target_release,
                },
                source_mount: args.source_mount,
                target_mount: args.target_mount,
                tenant_id: args.tenant_id,
                deployer_service_account: args.deployer_service_account,
            })
        }
        Action::Vault(VaultAction::RestoreDrill(args)) => {
            if !args.yes {
                bail!("vault restore-drill requires --yes and a disposable target namespace");
            }
            vault::restore_drill(&vault::RestoreDrill {
                target: vault::Target {
                    context: args.target.context,
                    namespace: args.target.namespace,
                    release: args.target.release,
                },
                bucket: args.bucket,
                prefix: args.prefix,
                region: args.region,
                expected_mount: args.expected_mount,
                backup_service_account: args.backup_service_account,
            })
            .await
        }
        Action::Infrastructure(InfrastructureAction::Aws(AwsAction::EnsureVault(args))) => {
            cloud::ensure_vault(&cloud::EnsureVault {
                cluster_name: args.cluster_name,
                region: args.region,
                namespace: args.namespace,
                release: args.release,
                retention_days: args.retention_days,
            })
            .await
        }
    }
}

fn vault_initialize(args: VaultInitialize) -> vault::Initialize {
    vault::Initialize {
        target: vault::Target {
            context: args.target.context,
            namespace: args.target.namespace,
            release: args.target.release,
        },
        tenant_id: args.tenant_id,
        mount: args.mount,
        connectors_service_account: args.connectors_service_account,
        deployer_service_account: args.deployer_service_account,
        backup_service_account: args.backup_service_account,
    }
}

fn leak_check(args: &LeakCheck) -> Result<()> {
    let markers = leak::read_markers(&args.deny_file)?;
    let findings = leak::scan(&args.root, &markers, Some(&args.deny_file))?;
    for finding in &findings {
        println!(
            "{}:{}: confidential marker #{}",
            finding.path.display(),
            finding.line,
            finding.marker
        );
    }
    if findings.is_empty() {
        println!("confidential-marker check: clean");
        Ok(())
    } else {
        bail!(
            "confidential-marker check found {} occurrence(s)",
            findings.len()
        )
    }
}

fn render(args: &HelmTarget) -> Result<()> {
    require_file(&args.values)?;
    let mut command = Command::new("helm");
    command.args([
        "template",
        &args.target.release,
        &args.chart,
        "--namespace",
        &args.target.namespace,
        "--values",
    ]);
    command.arg(&args.values);
    if let Some(version) = &args.version {
        command.args(["--version", version]);
    }
    run_streaming(&mut command)
}

fn preflight(args: &ClusterTarget) -> Result<()> {
    let mut version = kubectl(&args.target);
    version.args(["version", "--client"]);
    run_streaming(&mut version)?;

    let mut nodes = kubectl(&args.target);
    nodes.args([
        "get",
        "nodes",
        "-l",
        &format!("kubernetes.io/arch={}", args.required_architecture),
        "-o",
        "name",
    ]);
    let output = run_capture(&mut nodes)?;
    if output.trim().is_empty() {
        bail!(
            "no node has required architecture {}",
            args.required_architecture
        );
    }
    println!("preflight: cluster and node architecture available");
    Ok(())
}

fn apply(args: &HelmTarget) -> Result<()> {
    require_file(&args.values)?;
    let initialization = if args.initialize_vault {
        Some(vault::Initialize {
            target: vault::Target {
                context: args.target.context.clone(),
                namespace: args.target.namespace.clone(),
                release: args.target.release.clone(),
            },
            tenant_id: args
                .vault_tenant_id
                .clone()
                .context("--vault-tenant-id is required with --initialize-vault")?,
            mount: args.vault_mount.clone(),
            connectors_service_account: args
                .vault_connectors_service_account
                .clone()
                .context("--vault-connectors-service-account is required")?,
            deployer_service_account: args
                .vault_deployer_service_account
                .clone()
                .context("--vault-deployer-service-account is required")?,
            backup_service_account: args
                .vault_backup_service_account
                .clone()
                .context("--vault-backup-service-account is required")?,
        })
    } else {
        None
    };
    if let Some(initialization) = initialization {
        let mut bootstrap = helm_apply_command(args);
        run_streaming(&mut bootstrap)?;
        let mut wait = kubectl(&args.target);
        wait.args([
            "-n",
            &args.target.namespace,
            "wait",
            "--for=jsonpath={.status.phase}=Running",
            &format!("pod/{}-vault-0", args.target.release),
            "--timeout=10m",
        ]);
        run_streaming(&mut wait)?;
        vault::initialize(&initialization)?;
    }
    let mut command = helm_apply_command(args);
    command.args(["--atomic", "--wait"]);
    run_streaming(&mut command)
}

fn helm_apply_command(args: &HelmTarget) -> Command {
    let mut command = Command::new("helm");
    command.args([
        "upgrade",
        "--install",
        &args.target.release,
        &args.chart,
        "--namespace",
        &args.target.namespace,
    ]);
    if args.create_namespace {
        command.arg("--create-namespace");
    }
    command.arg("--values").arg(&args.values);
    command.args(["--timeout", &args.timeout, "--history-max", "10"]);
    if let Some(version) = &args.version {
        command.args(["--version", version]);
    }
    if let Some(context) = &args.target.context {
        command.args(["--kube-context", context]);
    }
    command
}

fn status(args: &ReleaseTarget) -> Result<()> {
    let mut command = Command::new("helm");
    command.args(["status", &args.release, "--namespace", &args.namespace]);
    if let Some(context) = &args.context {
        command.args(["--kube-context", context]);
    }
    run_streaming(&mut command)
}

fn verify(args: &Verify) -> Result<()> {
    for path in [
        "/healthz",
        "/readyz",
        "/docs/",
        "/openapi.json",
        "/.well-known/oauth-protected-resource",
    ] {
        let url = format!("{}{}", args.origin.trim_end_matches('/'), path);
        let mut command = Command::new("curl");
        command.args([
            "--fail",
            "--silent",
            "--show-error",
            "--max-time",
            "10",
            &url,
        ]);
        command.stdout(Stdio::null());
        run_streaming(&mut command).with_context(|| format!("verification failed for {path}"))?;
    }
    println!("verification: HTTP surface is ready");
    Ok(())
}

fn rollback(args: &Rollback) -> Result<()> {
    if !args.yes {
        bail!("rollback requires --yes and an explicit --revision");
    }
    if vault_enabled_at_revision(&args.target, None)?
        && !vault_enabled_at_revision(&args.target, Some(args.revision))?
        && !args.allow_secret_store_removal
    {
        bail!(
            "rollback would remove the active secret store; pass --allow-secret-store-removal only after credential migration or recovery"
        );
    }
    let mut command = Command::new("helm");
    command.args([
        "rollback",
        &args.target.release,
        &args.revision.to_string(),
        "--namespace",
        &args.target.namespace,
        "--wait",
        "--timeout",
        &args.timeout,
    ]);
    if let Some(context) = &args.target.context {
        command.args(["--kube-context", context]);
    }
    run_streaming(&mut command)
}

fn vault_enabled_at_revision(target: &ReleaseTarget, revision: Option<u32>) -> Result<bool> {
    let mut command = Command::new("helm");
    command.args([
        "get",
        "values",
        &target.release,
        "--namespace",
        &target.namespace,
        "--output",
        "json",
    ]);
    if let Some(revision) = revision {
        command.args(["--revision", &revision.to_string()]);
    }
    if let Some(context) = &target.context {
        command.args(["--kube-context", context]);
    }
    let values: serde_json::Value = serde_json::from_str(&run_capture(&mut command)?)
        .context("Helm returned invalid release values")?;
    Ok(values
        .pointer("/secretStore/vault/enabled")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false))
}

fn mirror(args: &Mirror) -> Result<()> {
    if !args.source.contains("@sha256:") {
        bail!("source must be digest-pinned");
    }
    if !args.destination.contains("@sha256:") {
        bail!("destination must name the expected digest");
    }
    let mut command = Command::new("oras");
    command.args(["cp", &args.source, &args.destination]);
    run_streaming(&mut command)
}

fn release_verify(args: &ReleaseVerify) -> Result<()> {
    if !args.expected_digest.starts_with("sha256:") {
        bail!("expected digest must use sha256");
    }
    let mut command = Command::new("oras");
    command.args(["manifest", "fetch", "--descriptor", &args.artifact]);
    let output = run_capture(&mut command)?;
    let descriptor: serde_json::Value =
        serde_json::from_str(&output).context("invalid OCI descriptor")?;
    let actual = descriptor.get("digest").and_then(serde_json::Value::as_str);
    if actual != Some(args.expected_digest.as_str()) {
        bail!("artifact digest does not match deployment lock");
    }
    println!("release verification: digest matches");
    Ok(())
}

fn bundle_validate(args: &BundleValidate) -> Result<()> {
    if !args.root.is_dir() {
        bail!("bundle root {} is not a directory", args.root.display());
    }
    let pack = args.root.join("catalog.pack");
    require_file(&pack)?;
    let metadata = std::fs::metadata(&pack).context("cannot inspect catalog.pack")?;
    if metadata.len() == 0 {
        bail!("catalog.pack is empty");
    }
    for entry in walkdir::WalkDir::new(&args.root).follow_links(false) {
        let entry = entry.context("cannot walk bundle")?;
        if entry.file_type().is_symlink() {
            bail!("bundle contains symlink {}", entry.path().display());
        }
    }
    println!("connector bundle: structurally valid");
    Ok(())
}

fn kubectl(target: &ReleaseTarget) -> Command {
    let mut command = Command::new("kubectl");
    if let Some(context) = &target.context {
        command.args(["--context", context]);
    }
    command
}

fn require_file(path: &Path) -> Result<()> {
    if path.is_file() {
        Ok(())
    } else {
        bail!("required file {} does not exist", path.display())
    }
}

fn run_streaming(command: &mut Command) -> Result<()> {
    let program = command.get_program().to_string_lossy().into_owned();
    let status = command
        .status()
        .with_context(|| format!("cannot run {program}"))?;
    if status.success() {
        Ok(())
    } else {
        bail!("{program} exited with {status}")
    }
}

fn run_capture(command: &mut Command) -> Result<String> {
    let program = command.get_program().to_string_lossy().into_owned();
    let output = command
        .output()
        .with_context(|| format!("cannot run {program}"))?;
    if !output.status.success() {
        bail!("{program} exited with {}", output.status);
    }
    String::from_utf8(output.stdout).context("command output is not UTF-8")
}
