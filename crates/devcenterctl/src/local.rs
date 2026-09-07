//! Local Kubernetes composition, with an explicit context and retained failure evidence.

use anyhow::{Context, Result, bail, ensure};
use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::deployment::{DeploymentLock, validate_rendered};

#[path = "local_bootstrap.rs"]
mod bootstrap;
#[path = "local_build.rs"]
mod build;

#[derive(Debug, Subcommand)]
pub enum LocalAction {
    /// Build selected candidates, prepare the real local stack, and run browser acceptance.
    Up(build::Up),
    /// Create an isolated local cluster without switching the user's current context.
    Create(Create),
    /// Inspect the exact owned cluster and report actual node conditions.
    Doctor(Target),
    /// Load a locally built image into every node of the owned cluster.
    Load(Load),
    /// Validate the complete composition and install the candidate chart locally.
    Apply(Apply),
    /// Prepare local credentials and fixtures from explicit deployment inputs.
    Prepare(bootstrap::Prepare),
    /// Run the repository's real-service browser acceptance checks.
    Test(Test),
    /// Delete only the cluster whose container identity is recorded in this state directory.
    Down(Target),
}

#[derive(Debug, Args)]
pub struct Target {
    /// Private local run directory containing the kubeconfig and evidence.
    #[arg(long)]
    pub state: PathBuf,
}

#[derive(Debug, Args)]
pub struct Create {
    #[command(flatten)]
    target: Target,
    #[arg(long, default_value = "devcenter-acceptance")]
    cluster: String,
    /// Pinned K3s node image, chosen by the local environment.
    #[arg(long)]
    node_image: String,
    #[arg(long, default_value_t = 16550)]
    api_port: u16,
    #[arg(long, default_value_t = 18080)]
    http_port: u16,
    #[arg(long, default_value_t = 18443)]
    https_port: u16,
    /// Record an existing cluster only when this exact Docker container ID matches.
    #[arg(long)]
    adopt_container_id: Option<String>,
    /// Registry created and owned by this local environment.
    #[arg(long)]
    registry: Option<String>,
    #[arg(skip)]
    profile_source: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct Load {
    #[command(flatten)]
    target: Target,
    #[arg(long, required = true)]
    image: Vec<String>,
}

#[derive(Debug, Args)]
pub struct Apply {
    #[command(flatten)]
    target: Target,
    #[arg(long)]
    chart: PathBuf,
    #[arg(long)]
    values: PathBuf,
    #[arg(long)]
    lock: PathBuf,
    #[arg(long, default_value = "devcenter")]
    namespace: String,
    #[arg(long, default_value = "devcenter")]
    release: String,
    #[arg(long, default_value = "5m")]
    timeout: String,
}

#[derive(Debug, Args)]
pub struct Test {
    #[command(flatten)]
    target: Target,
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    origin: String,
    /// Owner-only browser session issued through this local Identity's normal flow.
    #[arg(long)]
    storage_state: PathBuf,
    #[arg(long)]
    project: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnedCluster {
    schema: u32,
    cluster: String,
    container_id: String,
    node_image: String,
    api_port: u16,
    http_port: u16,
    https_port: u16,
}

pub fn run(action: LocalAction) -> Result<()> {
    match action {
        LocalAction::Up(args) => build::up(&args),
        LocalAction::Create(args) => create(args),
        LocalAction::Doctor(target) => doctor(&target),
        LocalAction::Load(args) => {
            let owned = load_owned(&args.target.state)?;
            for image in args.image {
                ensure!(
                    !image.starts_with('-') && !image.is_empty(),
                    "invalid image reference"
                );
                capture(
                    &args.target.state,
                    "image-import",
                    Command::new("k3d").args([
                        "image",
                        "import",
                        &image,
                        "--cluster",
                        &owned.cluster,
                    ]),
                )?;
            }
            Ok(())
        }
        LocalAction::Apply(args) => apply(&args),
        LocalAction::Prepare(args) => bootstrap::prepare(&args),
        LocalAction::Test(args) => build::test(&args),
        LocalAction::Down(target) => {
            let owned = load_owned(&target.state)?;
            build::down(&target.state)?;
            capture(
                &target.state,
                "cluster-delete",
                Command::new("k3d").args(["cluster", "delete", &owned.cluster]),
            )?;
            fs::rename(
                target.state.join("local.json"),
                target.state.join("retired.json"),
            )?;
            let kubeconfig = target.state.join("kubeconfig");
            if kubeconfig.exists() {
                fs::remove_file(kubeconfig)?;
            }
            println!(
                "removed owned cluster {}; retained evidence at {}",
                owned.cluster,
                target.state.display()
            );
            Ok(())
        }
    }
}

fn create(args: Create) -> Result<()> {
    ensure!(
        valid_name(&args.cluster),
        "cluster must be a lowercase DNS label beginning devcenter-"
    );
    ensure!(
        [args.api_port, args.http_port, args.https_port]
            .iter()
            .all(|p| *p >= 1024),
        "local ports must be unprivileged"
    );
    ensure!(
        args.api_port != args.http_port
            && args.api_port != args.https_port
            && args.http_port != args.https_port,
        "local ports must be distinct"
    );
    fs::create_dir_all(&args.target.state)?;
    fs::set_permissions(&args.target.state, fs::Permissions::from_mode(0o700))?;
    if args.target.state.join("local.json").exists() {
        let owned = load_owned(&args.target.state)?;
        ensure!(
            owned.cluster == args.cluster
                && owned.node_image == args.node_image
                && owned.api_port == args.api_port
                && owned.http_port == args.http_port
                && owned.https_port == args.https_port,
            "existing local installation has different immutable settings"
        );
        return Ok(());
    }
    let name = format!("k3d-{}-server-0", args.cluster);
    if let Some(expected) = &args.adopt_container_id {
        let actual = container_id(&name)?;
        ensure!(
            &actual == expected,
            "existing container identity does not match explicit adoption"
        );
    } else {
        ensure!(
            container_id(&name).is_err(),
            "cluster already exists; an explicit exact-ID adoption is required"
        );
        let mut command = Command::new("k3d");
        command.args(["cluster", "create", &args.cluster, "--image", &args.node_image, "--servers", "1", "--agents", "0", "--servers-memory", "12g", "--api-port", &format!("127.0.0.1:{}", args.api_port), "--port", &format!("127.0.0.1:{}:80@loadbalancer", args.http_port), "--port", &format!("127.0.0.1:{}:443@loadbalancer", args.https_port), "--volume", "/sys/kernel/security:/sys/kernel/security@server:0", "--k3s-arg", "--kubelet-arg=eviction-hard=nodefs.available<10Gi,imagefs.available<10Gi,nodefs.inodesFree<1%,imagefs.inodesFree<1%@server:0", "--k3s-arg", "--kubelet-arg=eviction-minimum-reclaim=nodefs.available=1Gi,imagefs.available=1Gi@server:0", "--k3s-arg", "--kubelet-arg=image-gc-high-threshold=99@server:0", "--k3s-arg", "--kubelet-arg=image-gc-low-threshold=98@server:0", "--kubeconfig-update-default=false", "--kubeconfig-switch-context=false", "--timeout", "120s"]);
        if let Some(registry) = &args.registry {
            ensure!(
                registry.starts_with("k3d-devcenter-") && registry.ends_with(":5000"),
                "registry must be local infrastructure"
            );
            command.args(["--registry-use", registry]);
        }
        if let Some(source) = &args.profile_source {
            ensure!(
                source == &args.target.state.join("node-profiles"),
                "profile receipt must remain in this local state"
            );
            fs::create_dir_all(source)?;
            command.args([
                "--volume",
                &format!(
                    "{}:/var/lib/kubelet/seccomp/substrate@server:0",
                    fs::canonicalize(source)?.display()
                ),
            ]);
        }
        capture(&args.target.state, "cluster-create", &mut command)?;
    }
    let owned = OwnedCluster {
        schema: 1,
        cluster: args.cluster.clone(),
        container_id: container_id(&name)?,
        node_image: args.node_image,
        api_port: args.api_port,
        http_port: args.http_port,
        https_port: args.https_port,
    };
    let output = Command::new("k3d")
        .args(["kubeconfig", "get", &args.cluster])
        .output()?;
    ensure!(
        output.status.success(),
        "could not obtain the owned cluster's kubeconfig"
    );
    write_private(&args.target.state.join("kubeconfig"), &output.stdout)?;
    write_private(
        &args.target.state.join("local.json"),
        &serde_json::to_vec_pretty(&owned)?,
    )?;
    println!(
        "local cluster ready; HTTPS port {}; state {}",
        args.https_port,
        args.target.state.display()
    );
    Ok(())
}

fn doctor(target: &Target) -> Result<()> {
    let owned = load_owned(&target.state)?;
    let output = capture(
        &target.state,
        "doctor",
        kube(&target.state).args(["get", "nodes", "-o", "json"]),
    )?;
    let nodes: serde_json::Value = serde_json::from_slice(&output)?;
    println!("cluster: {}", owned.cluster);
    let items = nodes["items"].as_array().context("node list missing")?;
    ensure!(!items.is_empty(), "local cluster has no nodes");
    for node in items {
        println!("{}", node["metadata"]["name"].as_str().unwrap_or("unknown"));
        for condition in node["status"]["conditions"]
            .as_array()
            .context("node conditions missing")?
        {
            println!(
                "  {}: {}",
                condition["type"].as_str().unwrap_or("unknown"),
                condition["status"].as_str().unwrap_or("unknown")
            );
        }
        let conditions = node["status"]["conditions"]
            .as_array()
            .context("node conditions missing")?;
        ensure!(
            conditions
                .iter()
                .any(|c| c["type"] == "Ready" && c["status"] == "True"),
            "local node is not Ready"
        );
        ensure!(
            !conditions
                .iter()
                .any(|c| ["DiskPressure", "MemoryPressure", "PIDPressure"]
                    .iter()
                    .any(|t| c["type"] == *t)
                    && c["status"] != "False"),
            "local node reports resource pressure"
        );
    }
    Ok(())
}

fn apply(args: &Apply) -> Result<()> {
    load_owned(&args.target.state)?;
    let values: serde_yaml::Value = serde_yaml::from_slice(&fs::read(&args.values)?)?;
    validate_local_origin(
        values["global"]["publicOrigin"]
            .as_str()
            .context("local public origin missing")?,
    )?;
    let lock = DeploymentLock::read(&args.lock)?;
    let rendered = capture(
        &args.target.state,
        "chart-render",
        Command::new("helm")
            .args(["template", &args.release])
            .arg(&args.chart)
            .args(["--namespace", &args.namespace, "--values"])
            .arg(&args.values),
    )?;
    validate_rendered(
        &lock,
        std::str::from_utf8(&rendered)?,
        &[
            "identity".into(),
            "connectors".into(),
            "workspace".into(),
            "substrate".into(),
            "agent-platform".into(),
        ],
    )?;
    capture(
        &args.target.state,
        "chart-apply",
        Command::new("helm")
            .args(["upgrade", "--install", &args.release])
            .arg(&args.chart)
            .arg("--kubeconfig")
            .arg(args.target.state.join("kubeconfig"))
            .args([
                "--namespace",
                &args.namespace,
                "--create-namespace",
                "--values",
            ])
            .arg(&args.values)
            .args(["--wait", "--timeout", &args.timeout]),
    )?;
    println!("local chart installed; browser acceptance is still required");
    Ok(())
}

fn valid_name(name: &str) -> bool {
    name.starts_with("devcenter-")
        && name.len() <= 40
        && !name.ends_with('-')
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn validate_local_origin(origin: &str) -> Result<()> {
    let parsed = url::Url::parse(origin)?;
    let host = parsed.host_str().context("local origin host missing")?;
    ensure!(
        parsed.scheme() == "https"
            && parsed.path() == "/"
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.query().is_none()
            && parsed.fragment().is_none()
            && (host == "localhost"
                || host.ends_with(".localhost")
                || host.rsplit('.').next() == Some("test")),
        "local origin must be an HTTPS origin on a reserved test domain"
    );
    Ok(())
}

fn load_owned(state: &Path) -> Result<OwnedCluster> {
    private_file(&state.join("kubeconfig"))?;
    let owned: OwnedCluster = serde_json::from_slice(&fs::read(state.join("local.json"))?)?;
    ensure!(
        owned.schema == 1 && valid_name(&owned.cluster),
        "invalid local ownership record"
    );
    ensure!(
        container_id(&format!("k3d-{}-server-0", owned.cluster))? == owned.container_id,
        "cluster has been replaced; refusing to use a different installation"
    );
    let config: serde_yaml::Value = serde_yaml::from_slice(&fs::read(state.join("kubeconfig"))?)?;
    let expected = format!("https://127.0.0.1:{}", owned.api_port);
    let clusters = config["clusters"]
        .as_sequence()
        .context("kubeconfig clusters missing")?;
    ensure!(
        clusters.len() == 1 && clusters[0]["cluster"]["server"].as_str() == Some(expected.as_str()),
        "kubeconfig must target only the recorded loopback API endpoint"
    );
    Ok(owned)
}

fn container_id(name: &str) -> Result<String> {
    let output = Command::new("docker")
        .args(["inspect", "--format", "{{.Id}}", name])
        .output()?;
    ensure!(output.status.success(), "owned Docker node is absent");
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn kube(state: &Path) -> Command {
    let mut command = Command::new("kubectl");
    command
        .arg("--kubeconfig")
        .arg(state.join("kubeconfig"))
        .arg("--request-timeout=15s");
    command
}

fn private_file(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    ensure!(
        metadata.is_file() && metadata.permissions().mode().trailing_zeros() >= 6,
        "{} must be an owner-only regular file",
        path.display()
    );
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8]) -> Result<()> {
    if path.symlink_metadata().is_ok() {
        private_file(path)?;
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(bytes)?;
    Ok(())
}

fn capture(state: &Path, phase: &str, command: &mut Command) -> Result<Vec<u8>> {
    let started = Instant::now();
    let attempt = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let phase = format!("{phase}-{attempt}");
    let stdout_path = state.join(format!("{phase}.stdout.log"));
    let stderr_path = state.join(format!("{phase}.stderr.log"));
    let log = |path: &Path| -> Result<fs::File> {
        Ok(OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)?)
    };
    let mut child = command
        .stdin(Stdio::null())
        .stdout(log(&stdout_path)?)
        .stderr(log(&stderr_path)?)
        .process_group(0)
        .spawn()
        .with_context(|| format!("start {phase}"))?;
    let limit = if phase.starts_with("build-") {
        1800
    } else {
        600
    };
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed().as_secs() > limit
            || fs::metadata(&stdout_path)?.len() + fs::metadata(&stderr_path)?.len()
                > 512 * 1024 * 1024
        {
            let group = format!("-{}", child.id());
            let _ = Command::new("kill").args(["-TERM", "--", &group]).status();
            std::thread::sleep(std::time::Duration::from_secs(1));
            let _ = Command::new("kill").args(["-KILL", "--", &group]).status();
            let _ = child.wait();
            bail!(
                "{phase} exceeded time or output limit; evidence: {}",
                state.display()
            );
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    };
    println!(
        "{phase}: {status} ({:.1}s)",
        started.elapsed().as_secs_f64()
    );
    ensure!(
        status.success(),
        "{phase} failed; evidence: {}",
        state.display()
    );
    ensure!(
        fs::metadata(&stdout_path)?.len() <= 64 * 1024 * 1024,
        "{phase} output exceeds parser limit; evidence retained"
    );
    Ok(fs::read(stdout_path)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hosted_and_misleading_origins_cannot_select_local_acceptance() {
        for origin in [
            "https://production.example.com",
            "http://localhost:18443",
            "https://localhost@production.example.com",
            "https://local.test/path",
            "https://local.test?redirect=live",
        ] {
            assert!(validate_local_origin(origin).is_err(), "{origin}");
        }
        assert!(validate_local_origin("https://devcenter.localhost:18443").is_ok());
    }

    #[test]
    fn cluster_names_are_explicitly_scoped() {
        for name in [
            "production",
            "devcenter-",
            "devcenter-X",
            "devcenter-test/other",
            "--all",
        ] {
            assert!(!valid_name(name));
        }
        assert!(valid_name("devcenter-acceptance"));
    }
}
