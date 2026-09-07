//! Cached local builds and the complete acceptance cycle.
use super::{
    Apply, Create, Target, bootstrap, capture, container_id, create, doctor, kube, load_owned,
    private_file, write_private,
};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use clap::Args;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, path::PathBuf, process::Command};

#[derive(Debug, Args)]
pub struct Up {
    #[command(flatten)]
    target: Target,
    #[arg(long)]
    source: PathBuf,
    #[arg(long)]
    baseline_values: PathBuf,
    #[arg(long)]
    baseline_lock: PathBuf,
    /// Modified Identity checkout; omitted to use the pinned baseline image.
    #[arg(long)]
    identity_source: Option<PathBuf>,
    /// Components whose current checkout should be built. Others retain their baseline digest.
    #[arg(long, value_parser=["server","connectors"])]
    build: Vec<String>,
    #[arg(long, default_value = "b10x-local")]
    builder: String,
    /// A pinned K3s base image. Local prerequisites are added in a separate infrastructure image.
    #[arg(long)]
    k3s_image: String,
    /// Owner-only Docker client configuration directory for baseline image pulls.
    #[arg(long)]
    docker_config: PathBuf,
    /// Owner-only ephemeral GitHub build credential; never copied into build arguments or values.
    #[arg(long)]
    github_token_file: Option<PathBuf>,
    /// Adopt only this exact previously created registry container.
    #[arg(long)]
    adopt_registry_id: Option<String>,
    #[arg(long)]
    adopt_forwarder_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Infrastructure {
    registry_id: String,
    forwarder_id: Option<String>,
}

pub fn up(args: &Up) -> Result<()> {
    let state = &args.target.state;
    fs::create_dir_all(state)?;
    fs::set_permissions(state, fs::Permissions::from_mode(0o700))?;
    private_file(&args.docker_config.join("config.json"))?;
    ensure!(
        args.k3s_image.contains("@sha256:"),
        "K3s base must use an immutable digest"
    );
    let baseline: Value = serde_yaml::from_slice(&fs::read(&args.baseline_values)?)?;
    let substrate = image(&baseline["substrate"]["image"])?;
    build_examples(args)?;
    let provider = docker_build(
        args,
        "provider",
        &args.source.join("ci/local/Dockerfile.provider"),
        &[format!("RUNTIME_IMAGE={substrate}")],
    )?;
    let mut infrastructure = registry(args)?;
    if !state.join("local.json").exists() {
        let node = docker_build(
            args,
            "node",
            &args.source.join("ci/local/Dockerfile.node"),
            &[
                format!("SUBSTRATE_IMAGE={substrate}"),
                format!("K3S_IMAGE={}", args.k3s_image),
            ],
        )?;
        create(Create {
            target: Target {
                state: state.clone(),
            },
            cluster: "devcenter-acceptance".into(),
            node_image: node,
            api_port: 16550,
            http_port: 18080,
            https_port: 18443,
            adopt_container_id: None,
            registry: Some("k3d-devcenter-registry.localhost:5000".into()),
            profile_source: Some(state.join("node-profiles")),
        })?;
    }
    republish_local_baseline(args)?;
    let owned = load_owned(state)?;
    ensure!(
        owned.cluster == "devcenter-acceptance",
        "complete local cycle requires its dedicated default cluster"
    );
    doctor(&args.target)?;
    pull_secret(args)?;
    install_profiles(state, &substrate)?;
    let provider = push(args, &provider)?;
    let (identity, connectors) = candidates(args, &baseline)?;
    forwarder(args, &provider, &mut infrastructure)?;
    bootstrap::prepare(&bootstrap::Prepare {
        target: Target {
            state: state.clone(),
        },
        baseline_values: args.baseline_values.clone(),
        baseline_lock: args.baseline_lock.clone(),
        identity_image: identity,
        connectors_image: connectors,
        provider_image: provider,
    })?;
    if args.build.iter().any(|c| c == "server") {
        select_server(state)?;
    }
    capture(
        state,
        "fixture-rollout",
        kube(state).args([
            "-n",
            "devcenter",
            "rollout",
            "status",
            "deployment/devcenter-local-provider",
            "--timeout=60s",
        ]),
    )?;
    super::apply(&Apply {
        target: Target {
            state: state.clone(),
        },
        chart: args.source.join("deploy/charts/devcenter"),
        values: state.join("values.local.yaml"),
        lock: state.join("deployment.local.lock.toml"),
        namespace: "devcenter".into(),
        release: "devcenter".into(),
        timeout: "3m".into(),
    })?;
    browser(args)?;
    println!("Local fixture acceptance passed: https://devcenter.localhost:18443");
    Ok(())
}

fn candidates(args: &Up, baseline: &Value) -> Result<(String, String)> {
    let state = &args.target.state;
    let mut identity = image(&baseline["components"]["identity"]["image"])?;
    if let Some(source) = &args.identity_source {
        let tag = "localhost:15000/identity:candidate";
        capture(
            state,
            "build-identity",
            docker(args)
                .args([
                    "buildx",
                    "build",
                    "--builder",
                    &args.builder,
                    "--load",
                    "-t",
                    tag,
                ])
                .arg(source),
        )?;
        identity = push(args, tag)?;
    }
    let mut connectors = image(&baseline["components"]["connectors"]["image"])?;
    for component in &args.build {
        let token = args
            .github_token_file
            .as_ref()
            .context("selected builds require --github-token-file")?;
        private_file(token)?;
        let tag = format!("localhost:15000/{component}:candidate");
        capture(
            state,
            &format!("build-{component}"),
            docker(args).current_dir(&args.source).args([
                "buildx",
                "bake",
                "--builder",
                &args.builder,
                "--file",
                "docker-bake.hcl",
                component,
                "--load",
                "--set",
                &format!("{component}.platform=linux/amd64"),
                "--set",
                &format!("{component}.tags={tag}"),
                "--set",
                &format!(
                    "{component}.secrets=id=github-token,src={}",
                    token.display()
                ),
            ]),
        )?;
        let pinned = push(args, &tag)?;
        if component == "connectors" {
            connectors = pinned;
        } else {
            write_private(&state.join("server-candidate.txt"), pinned.as_bytes())?;
        }
    }
    Ok((identity, connectors))
}

fn docker(args: &Up) -> Command {
    let mut command = Command::new("docker");
    command.arg("--config").arg(&args.docker_config);
    // Authentication stays in the private config; builder discovery retains the caller's context.
    if std::env::var_os("BUILDX_CONFIG").is_none() {
        let config = std::env::var_os("DOCKER_CONFIG")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".docker")));
        if let Some(config) = config {
            command.env("BUILDX_CONFIG", config.join("buildx"));
        }
    }
    command
}

fn image(value: &Value) -> Result<String> {
    Ok(format!(
        "{}@{}",
        value["repository"]
            .as_str()
            .context("image repository missing")?,
        value["digest"].as_str().context("image digest missing")?
    ))
}

fn build_examples(args: &Up) -> Result<()> {
    capture(
        &args.target.state,
        "build-local-tools",
        Command::new("cargo")
            .current_dir(&args.source)
            .args([
                "build",
                "--locked",
                "-p",
                "devcenterctl",
                "--example",
                "local-provider",
                "--example",
                "local-node",
                "--target-dir",
            ])
            .arg(args.target.state.join("ctl-target")),
    )?;
    for (directory, binary) in [("provider", "local-provider"), ("node", "local-node")] {
        let context = args.target.state.join(format!("{directory}-image"));
        fs::create_dir_all(&context)?;
        fs::copy(
            args.target
                .state
                .join("ctl-target/debug/examples")
                .join(binary),
            context.join(binary),
        )?;
        fs::set_permissions(context.join(binary), fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

fn docker_build(args: &Up, name: &str, file: &Path, build_args: &[String]) -> Result<String> {
    let tag = if name == "node" {
        "devcenter-local-node:candidate".to_owned()
    } else {
        format!("localhost:15000/{name}:candidate")
    };
    let mut command = docker(args);
    command.args(["build", "-t", &tag, "-f"]).arg(file);
    for arg in build_args {
        command.arg("--build-arg").arg(arg);
    }
    command.arg(args.target.state.join(format!("{name}-image")));
    capture(&args.target.state, &format!("build-{name}"), &mut command)?;
    Ok(tag)
}

fn push(args: &Up, tag: &str) -> Result<String> {
    let output = capture(
        &args.target.state,
        "push-local-image",
        docker(args).args(["push", tag]),
    )?;
    let digest = String::from_utf8(output)?
        .lines()
        .rev()
        .find_map(|line| {
            let words = line.split_ascii_whitespace().collect::<Vec<_>>();
            words.windows(2).find_map(|pair| {
                (pair[0] == "digest:" && pair[1].starts_with("sha256:") && pair[1].len() == 71)
                    .then(|| pair[1].to_owned())
            })
        })
        .context("Docker push did not return an immutable manifest digest")?;
    let repository = tag.rsplit_once(':').context("local push tag missing")?.0;
    Ok(format!(
        "{}@{digest}",
        repository.replace("localhost:15000/", "k3d-devcenter-registry.localhost:5000/")
    ))
}

fn republish_local_baseline(args: &Up) -> Result<()> {
    let lock = crate::deployment::DeploymentLock::read(&args.baseline_lock)?;
    for artifact in lock.images.values() {
        let Some(repository) = artifact
            .reference
            .strip_prefix("k3d-devcenter-registry.localhost:5000/")
        else {
            continue;
        };
        let source = format!("localhost:15000/{repository}@{}", artifact.digest);
        let tag = format!(
            "localhost:15000/{repository}:retained-{}",
            &artifact.digest[7..19]
        );
        capture(
            &args.target.state,
            "retain-local-image",
            docker(args).args(["tag", &source, &tag]),
        )?;
        ensure!(
            push(args, &tag)? == format!("{}@{}", artifact.reference, artifact.digest),
            "retained image manifest differs from the selected baseline"
        );
    }
    Ok(())
}

fn registry(args: &Up) -> Result<Infrastructure> {
    let file = args.target.state.join("infrastructure.json");
    let name = "k3d-devcenter-registry.localhost";
    if file.exists() {
        let record: Infrastructure = serde_json::from_slice(&fs::read(file)?)?;
        ensure!(
            container_id(name)? == record.registry_id,
            "local registry has been replaced"
        );
        return Ok(record);
    }
    if let Some(expected) = &args.adopt_registry_id {
        ensure!(
            container_id(name)? == *expected,
            "registry adoption ID differs"
        );
    } else {
        ensure!(
            container_id(name).is_err(),
            "existing registry requires exact-ID adoption"
        );
        capture(
            &args.target.state,
            "registry-create",
            Command::new("k3d").args([
                "registry",
                "create",
                "devcenter-registry.localhost",
                "--port",
                "127.0.0.1:15000",
                "--no-help",
            ]),
        )?;
    }
    let record = Infrastructure {
        registry_id: container_id(name)?,
        forwarder_id: None,
    };
    write_private(&file, &serde_json::to_vec(&record)?)?;
    Ok(record)
}

fn forwarder(args: &Up, image: &str, record: &mut Infrastructure) -> Result<()> {
    let name = "devcenter-local-https";
    if let Some(expected) = record
        .forwarder_id
        .as_ref()
        .or(args.adopt_forwarder_id.as_ref())
    {
        ensure!(
            container_id(name)? == *expected,
            "local HTTPS forwarder has been replaced"
        );
    } else {
        ensure!(
            container_id(name).is_err(),
            "existing HTTPS forwarder requires exact-ID adoption"
        );
        let image = image.replace("k3d-devcenter-registry.localhost:5000/", "localhost:15000/");
        capture(
            &args.target.state,
            "https-forwarder",
            docker(args).args([
                "run",
                "-d",
                "--name",
                name,
                "--network",
                "k3d-devcenter-acceptance",
                "-p",
                "127.0.0.1:443:8080",
                "--read-only",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges",
                "--memory",
                "64m",
                "--pids-limit",
                "32",
                "-e",
                "LOCAL_ACCEPTANCE_FIXTURE=1",
                &image,
                "--origin",
                "https://provider.devcenter.localhost",
                "--app-origin",
                "https://devcenter.localhost:18443",
                "--state",
                "/unused",
                "--forward",
                "k3d-devcenter-acceptance-serverlb:443",
            ]),
        )?;
    }
    record.forwarder_id = Some(container_id(name)?);
    write_private(
        &args.target.state.join("infrastructure.json"),
        &serde_json::to_vec(record)?,
    )?;
    Ok(())
}

fn pull_secret(args: &Up) -> Result<()> {
    let state = &args.target.state;
    let file = state.join("image-pull-secret.json");
    let secret = json!({"apiVersion":"v1","kind":"List","items":[{"apiVersion":"v1","kind":"Namespace","metadata":{"name":"devcenter"}},{"apiVersion":"v1","kind":"Secret","metadata":{"name":"github-container-registry","namespace":"devcenter"},"type":"kubernetes.io/dockerconfigjson","data":{".dockerconfigjson":STANDARD.encode(fs::read(args.docker_config.join("config.json"))?)}}]});
    write_private(&file, &serde_json::to_vec(&secret)?)?;
    let result = capture(
        state,
        "image-pull-secret",
        kube(state).args(["apply", "-f"]).arg(&file),
    );
    fs::remove_file(file)?;
    result.map(|_| ())
}

fn install_profiles(state: &Path, image: &str) -> Result<()> {
    let label = json!({"app.kubernetes.io/name":"devcenter-substrate-execution-profiles"});
    let file = state.join("execution-profiles.json");
    let resources = json!({"apiVersion":"v1","kind":"List","items":[{"apiVersion":"apps/v1","kind":"DaemonSet","metadata":{"name":"devcenter-substrate-execution-profiles","namespace":"devcenter"},"spec":{"selector":{"matchLabels":label},"template":{"metadata":{"labels":label},"spec":{"automountServiceAccountToken":false,"nodeSelector":{"kubernetes.io/arch":"amd64"},"imagePullSecrets":[{"name":"github-container-registry"}],"containers":[{"name":"profiles","image":image,"command":["/usr/local/bin/substrate-container-profiles"],"args":["--hold"],"readinessProbe":{"exec":{"command":["/usr/local/bin/substrate-container-profiles","--check"]},"timeoutSeconds":5},"securityContext":{"runAsUser":0,"runAsGroup":0,"allowPrivilegeEscalation":false,"readOnlyRootFilesystem":true,"appArmorProfile":{"type":"Unconfined"},"seccompProfile":{"type":"RuntimeDefault"},"capabilities":{"drop":["ALL"],"add":["MAC_ADMIN"]}},"volumeMounts":[{"name":"securityfs","mountPath":"/sys/kernel/security"},{"name":"profiles","mountPath":"/node-profiles"}]}],"volumes":[{"name":"securityfs","hostPath":{"path":"/sys/kernel/security","type":"Directory"}},{"name":"profiles","hostPath":{"path":"/var/lib/kubelet/seccomp/substrate","type":"DirectoryOrCreate"}}]}}}},{"apiVersion":"networking.k8s.io/v1","kind":"NetworkPolicy","metadata":{"name":"devcenter-substrate-execution-profiles","namespace":"devcenter"},"spec":{"podSelector":{"matchLabels":label},"policyTypes":["Ingress","Egress"],"ingress":[],"egress":[]}}]});
    write_private(&file, &serde_json::to_vec(&resources)?)?;
    capture(
        state,
        "execution-profiles",
        kube(state).args(["apply", "-f"]).arg(file),
    )?;
    capture(
        state,
        "execution-profiles-ready",
        kube(state).args([
            "-n",
            "devcenter",
            "rollout",
            "status",
            "daemonset/devcenter-substrate-execution-profiles",
            "--timeout=60s",
        ]),
    )?;
    Ok(())
}

fn select_server(state: &Path) -> Result<()> {
    let image = fs::read_to_string(state.join("server-candidate.txt"))?;
    let (repository, digest) = image
        .split_once('@')
        .context("candidate server digest missing")?;
    let mut values: Value = serde_yaml::from_slice(&fs::read(state.join("values.local.yaml"))?)?;
    values["devcenter"]["image"] = json!({"repository":repository,"digest":digest});
    write_private(
        &state.join("values.local.yaml"),
        serde_yaml::to_string(&values)?.as_bytes(),
    )?;
    let mut lock: toml::Value = toml::from_str(&fs::read_to_string(
        state.join("deployment.local.lock.toml"),
    )?)?;
    lock.get_mut("images")
        .and_then(toml::Value::as_table_mut)
        .context("deployment image lock missing")?
        .insert(
            "devcenter".into(),
            toml::Value::try_from(
                json!({"reference":repository,"digest":digest,"version":"local-candidate"}),
            )?,
        );
    write_private(
        &state.join("deployment.local.lock.toml"),
        toml::to_string_pretty(&lock)?.as_bytes(),
    )?;
    Ok(())
}

fn browser(args: &Up) -> Result<()> {
    let state = &args.target.state;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let evidence = state.join(format!("acceptance-{at}"));
    fs::create_dir(&evidence)?;
    let owned = load_owned(state)?;
    record_composition(args, &evidence)?;
    let previous = fs::read(state.join("last-project.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(|value| value["node_id"] == owned.container_id);
    let spki = tls_spki(state)?;
    for spec in ["setup.spec.ts", "history.spec.ts", "workspace.spec.ts"] {
        let mut command = Command::new("pnpm");
        command
            .arg("--dir")
            .arg(args.source.join("frontend"))
            .args([
                "exec",
                "playwright",
                "test",
                "--config",
                "playwright.acceptance.config.ts",
                spec,
            ])
            .env("NODE_EXTRA_CA_CERTS", state.join("ca.crt"))
            .env("DEVCENTER_LOCAL_TLS_SPKI", &spki)
            .env("DEVCENTER_PROVIDER_MODE", "fixture")
            .env(
                "DEVCENTER_PROVISIONING_FILE",
                state.join("fixture-provisioning.json"),
            )
            .env("DEVCENTER_ORIGIN", "https://devcenter.localhost:18443")
            .env("DEVCENTER_EVIDENCE_ROOT", &evidence);
        if let Some(expected) = previous
            .as_ref()
            .and_then(|value| value["project_id"].as_str())
        {
            command.env("DEVCENTER_EXPECTED_PROJECT_ID", expected);
        }
        if spec != "setup.spec.ts" {
            let project: Value = serde_json::from_slice(&fs::read(evidence.join("project.json"))?)?;
            command
                .env(
                    "PROJECTS_STORAGE_STATE",
                    evidence.join("storage-state.json"),
                )
                .env(
                    "DEVCENTER_PROJECT_ID",
                    project["id"]
                        .as_str()
                        .context("fixture project ID missing")?,
                );
        }
        capture(state, &format!("browser-{spec}"), &mut command)?;
    }
    let project: Value = serde_json::from_slice(&fs::read(evidence.join("project.json"))?)?;
    write_private(
        &state.join("last-project.json"),
        &serde_json::to_vec(&json!({"node_id":owned.container_id,"project_id":project["id"]}))?,
    )?;
    write_private(
        &state.join("last-acceptance.json"),
        &serde_json::to_vec(
            &json!({"result":"pass","provider_mode":"fixture","evidence":evidence,"origin":"https://devcenter.localhost:18443"}),
        )?,
    )?;
    Ok(())
}

fn record_composition(args: &Up, evidence: &Path) -> Result<()> {
    let state = &args.target.state;
    let lock = crate::deployment::DeploymentLock::read(&state.join("deployment.local.lock.toml"))?;
    let mut admitted = lock
        .images
        .values()
        .map(|image| {
            format!(
                "{}@{}",
                crate::deployment::canonical_image_reference(&image.reference),
                image.digest
            )
        })
        .collect::<std::collections::BTreeSet<_>>();
    admitted.insert(crate::deployment::canonical_image_reference(
        fs::read_to_string(state.join("provider-candidate.txt"))?.trim(),
    ));
    let pods: Value = serde_json::from_slice(&capture(
        state,
        "running-images",
        kube(state).args([
            "-n",
            "devcenter",
            "get",
            "pods",
            "-l",
            "app.kubernetes.io/instance=devcenter",
            "-o",
            "json",
        ]),
    )?)?;
    let mut running = Vec::new();
    for pod in pods["items"].as_array().context("running pods missing")? {
        for container in pod["spec"]["containers"]
            .as_array()
            .context("pod containers missing")?
        {
            ensure!(
                admitted.contains(&crate::deployment::canonical_image_reference(
                    container["image"].as_str().context("pod image missing")?
                )),
                "running workload differs from selected lock; replace its outdated pod before acceptance"
            );
        }
        let statuses = pod["status"]["containerStatuses"]
            .as_array()
            .context("container status missing")?;
        ensure!(
            !statuses.is_empty() && statuses.iter().all(|status| status["ready"] == true),
            "running workload is not ready"
        );
        running.push(json!({"pod":pod["metadata"]["name"],"uid":pod["metadata"]["uid"],"images":statuses.iter().map(|status| json!({"name":status["name"],"image":status["image"],"image_id":status["imageID"]})).collect::<Vec<_>>()}));
    }
    ensure!(!running.is_empty(), "no application workloads observed");
    for name in ["deployment.local.lock.toml", "local.json"] {
        write_private(&evidence.join(name), &fs::read(state.join(name))?)?;
    }
    write_private(
        &evidence.join("composition.json"),
        &serde_json::to_vec_pretty(
            &json!({"running":running,"source":args.source,"selected_builds":args.build,"provider_mode":"fixture"}),
        )?,
    )?;
    Ok(())
}

fn tls_spki(state: &Path) -> Result<String> {
    let pubkey = capture(
        state,
        "tls-public-key",
        Command::new("openssl")
            .args(["x509", "-in"])
            .arg(state.join("tls.crt"))
            .args(["-pubkey", "-noout"]),
    )?;
    write_private(&state.join("tls.pub"), &pubkey)?;
    let der = capture(
        state,
        "tls-spki-der",
        Command::new("openssl")
            .args(["pkey", "-pubin", "-in"])
            .arg(state.join("tls.pub"))
            .args(["-outform", "DER"]),
    )?;
    Ok(STANDARD.encode(Sha256::digest(der)))
}

pub(super) fn test(args: &super::Test) -> Result<()> {
    let state = &args.target.state;
    load_owned(state)?;
    super::validate_local_origin(&args.origin)?;
    private_file(&args.storage_state)?;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let output = state.join(format!("acceptance-{at}"));
    fs::create_dir(&output)?;
    capture(
        state,
        "browser-workspace",
        Command::new("pnpm")
            .arg("--dir")
            .arg(args.source.join("frontend"))
            .args([
                "exec",
                "playwright",
                "test",
                "--config",
                "playwright.acceptance.config.ts",
                "workspace.spec.ts",
            ])
            .env("NODE_EXTRA_CA_CERTS", state.join("ca.crt"))
            .env("DEVCENTER_LOCAL_TLS_SPKI", tls_spki(state)?)
            .env("DEVCENTER_ORIGIN", &args.origin)
            .env("PROJECTS_STORAGE_STATE", &args.storage_state)
            .env("DEVCENTER_PROJECT_ID", &args.project)
            .env("DEVCENTER_PROVIDER_MODE", "fixture")
            .env("DEVCENTER_EVIDENCE_ROOT", &output),
    )?;
    println!("browser acceptance passed: {}", output.display());
    Ok(())
}

pub(super) fn down(state: &Path) -> Result<()> {
    let file = state.join("infrastructure.json");
    if !file.exists() {
        return Ok(());
    }
    let record: Infrastructure = serde_json::from_slice(&fs::read(&file)?)?;
    ensure!(
        container_id("k3d-devcenter-registry.localhost")? == record.registry_id,
        "registry ownership differs"
    );
    if let Some(id) = &record.forwarder_id {
        ensure!(
            container_id("devcenter-local-https")? == *id,
            "forwarder ownership differs"
        );
    }
    if record.forwarder_id.is_some() {
        capture(
            state,
            "forwarder-stop",
            Command::new("docker").args(["stop", "devcenter-local-https"]),
        )?;
        capture(
            state,
            "forwarder-remove",
            Command::new("docker").args(["rm", "devcenter-local-https"]),
        )?;
    }
    capture(
        state,
        "registry-delete",
        Command::new("k3d").args(["registry", "delete", "devcenter-registry.localhost"]),
    )?;
    fs::rename(file, state.join("retired-infrastructure.json"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_candidate_updates_the_devcenter_lock_entry() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        let digest = format!("sha256:{}", "a".repeat(64));
        write_private(
            &state.join("server-candidate.txt"),
            format!("k3d-devcenter-registry.localhost:5000/server@{digest}").as_bytes(),
        )
        .unwrap();
        write_private(
            &state.join("values.local.yaml"),
            b"devcenter:\n  image: {}\n",
        )
        .unwrap();
        write_private(
            &state.join("deployment.local.lock.toml"),
            b"[images.devcenter]\nversion = 'old'\n",
        )
        .unwrap();
        select_server(state).unwrap();
        let lock: toml::Value =
            toml::from_str(&fs::read_to_string(state.join("deployment.local.lock.toml")).unwrap())
                .unwrap();
        assert_eq!(
            lock["images"]["devcenter"]["digest"].as_str(),
            Some(digest.as_str())
        );
        assert!(lock["images"].get("server").is_none());
    }
}
