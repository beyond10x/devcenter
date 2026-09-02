use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand};
use devcenterctl::deployment::{DeploymentLock, validate_rendered};
use devcenterctl::leak;
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
    Deployment(DeploymentAction),
    #[command(subcommand)]
    Bundle(BundleAction),
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
    #[arg(long, default_value = "5m")]
    timeout: String,
    #[arg(long)]
    create_namespace: bool,
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

#[derive(Debug, Subcommand)]
enum DeploymentAction {
    /// Render a chart and prove that all workloads match the deployment lock.
    Validate(DeploymentValidate),
}

#[derive(Debug, Args)]
struct DeploymentValidate {
    #[command(flatten)]
    target: HelmTarget,
    /// Immutable deployment lock to compare with the rendered chart.
    #[arg(long)]
    lock: PathBuf,
    /// Component that must be enabled in the rendered release. Repeat as needed.
    #[arg(long = "require-component")]
    required_components: Vec<String>,
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

fn main() -> Result<()> {
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
        Action::Deployment(DeploymentAction::Validate(args)) => deployment_validate(&args),
        Action::Bundle(BundleAction::Validate(args)) => bundle_validate(&args),
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

fn deployment_validate(args: &DeploymentValidate) -> Result<()> {
    require_file(&args.target.values)?;
    require_file(&args.lock)?;
    let lock = DeploymentLock::read(&args.lock)?;
    lock.validate_chart(&args.target.chart, args.target.version.as_deref())?;
    let rendered = render_capture(&args.target)?;
    validate_rendered(&lock, &rendered, &args.required_components)?;
    println!(
        "deployment validation: chart, {} workload image(s), and required components match the lock",
        lock.images.len()
    );
    Ok(())
}

fn render_capture(args: &HelmTarget) -> Result<String> {
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
    run_capture(&mut command)
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
    command.args([
        "--atomic",
        "--wait",
        "--timeout",
        &args.timeout,
        "--history-max",
        "10",
    ]);
    if let Some(version) = &args.version {
        command.args(["--version", version]);
    }
    if let Some(context) = &args.target.context {
        command.args(["--kube-context", context]);
    }
    let status = command.status().context("cannot run helm")?;
    if status.success() {
        Ok(())
    } else {
        collect_rollout_diagnostics(&args.target);
        bail!("helm exited with {status}; rollout diagnostics were emitted above")
    }
}

fn collect_rollout_diagnostics(target: &ReleaseTarget) {
    eprintln!("rollout diagnostics: workloads");
    let mut workloads = kubectl(target);
    workloads.args([
        "--namespace",
        &target.namespace,
        "get",
        "deployments,statefulsets,jobs,pods",
        "--selector",
        &format!("app.kubernetes.io/instance={}", target.release),
        "--output",
        "wide",
    ]);
    run_diagnostic(&mut workloads);

    eprintln!("rollout diagnostics: recent events");
    let mut events = kubectl(target);
    events.args([
        "--namespace",
        &target.namespace,
        "get",
        "events",
        "--sort-by=.metadata.creationTimestamp",
    ]);
    run_diagnostic(&mut events);

    eprintln!("rollout diagnostics: bounded container logs");
    let mut logs = kubectl(target);
    logs.args([
        "--namespace",
        &target.namespace,
        "logs",
        "--selector",
        &format!("app.kubernetes.io/instance={}", target.release),
        "--all-containers",
        "--prefix",
        "--tail=100",
    ]);
    run_diagnostic(&mut logs);
}

fn run_diagnostic(command: &mut Command) {
    if let Err(error) = command.status() {
        eprintln!("diagnostic command could not run: {error}");
    }
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
