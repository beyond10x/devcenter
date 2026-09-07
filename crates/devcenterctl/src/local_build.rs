//! Cached local builds and the complete acceptance cycle.
use super::integrations::{Options as IntegrationOptions, Selection as IntegrationSelection};
use super::{
    Apply, Create, LIVE_MODEL_ENDPOINT, Target, bootstrap, capture, container_id, create, doctor,
    kube, load_owned, private_file, write_private,
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
    integrations: IntegrationOptions,
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
    /// Modified Agent Platform checkout; omitted to use the pinned baseline image.
    #[arg(long)]
    agent_platform_source: Option<PathBuf>,
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
    acceptance_status(state, "not_completed", None)?;
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
        integrations: args.integrations.clone(),
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
        select_candidate(state, "server", "devcenter")?;
    }
    if args.agent_platform_source.is_some() {
        select_candidate(state, "agent-platform", "agent-platform")?;
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
    println!("Local acceptance with live Claude passed: https://devcenter.localhost:18443");
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
    agent_platform_candidate(args)?;
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

fn agent_platform_candidate(args: &Up) -> Result<()> {
    let state = &args.target.state;
    if let Some(source) = &args.agent_platform_source {
        let token = args
            .github_token_file
            .as_ref()
            .context("Agent Platform builds require --github-token-file")?;
        private_file(token)?;
        let sha = capture(
            state,
            "agent-platform-source",
            Command::new("git")
                .arg("-C")
                .arg(source)
                .args(["rev-parse", "HEAD"]),
        )?;
        let sha = String::from_utf8(sha)?.trim().to_owned();
        let dirty = capture(
            state,
            "agent-platform-changes",
            Command::new("git")
                .arg("-C")
                .arg(source)
                .args(["status", "--porcelain"]),
        )?;
        let tag = "localhost:15000/agent-platform:candidate";
        capture(
            state,
            "build-agent-platform",
            docker(args)
                .args([
                    "buildx",
                    "build",
                    "--builder",
                    &args.builder,
                    "--load",
                    "-t",
                    tag,
                    "--build-arg",
                    &format!("SOURCE_SHA={sha}"),
                    "--secret",
                    &format!("id=github_token,src={}", token.display()),
                ])
                .arg(source),
        )?;
        let image = push(args, tag)?;
        write_private(
            &state.join("agent-platform-candidate.txt"),
            image.as_bytes(),
        )?;
        write_private(
            &state.join("agent-platform-source.json"),
            &serde_json::to_vec(
                &json!({"path":source,"commit":sha,"dirty":!dirty.is_empty(),"image":image}),
            )?,
        )?;
    }
    Ok(())
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

fn select_candidate(state: &Path, candidate: &str, component: &str) -> Result<()> {
    let image = fs::read_to_string(state.join(format!("{candidate}-candidate.txt")))?;
    let (repository, digest) = image
        .split_once('@')
        .context("candidate server digest missing")?;
    let mut values: Value = serde_yaml::from_slice(&fs::read(state.join("values.local.yaml"))?)?;
    if component == "devcenter" {
        values["devcenter"]["image"] = json!({"repository":repository,"digest":digest});
    } else {
        values["components"][component]["image"] = json!({"repository":repository,"digest":digest});
    }
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
            component.into(),
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
    acceptance_status(state, "not_completed", Some(&evidence))?;
    let owned = load_owned(state)?;
    let integrations = IntegrationSelection::load(state)?;
    record_composition(state, &args.source, &args.build, &evidence)?;
    let previous = fs::read(state.join("last-project.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .filter(|value| {
            value["node_id"] == owned.container_id
                && value["connector_mode"].as_str().unwrap_or("fixture")
                    == integrations.mode.as_str()
        });
    let spki = tls_spki(state)?;
    let mut failures = Vec::new();
    for spec in [
        "setup.spec.ts",
        "integrations.spec.ts",
        "model.spec.ts",
        "history.spec.ts",
        "lifecycle.spec.ts",
        "workspace.spec.ts",
    ] {
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
            .env("DEVCENTER_PROVIDER_MODE", "live")
            .env("DEVCENTER_ORIGIN", "https://devcenter.localhost:18443")
            .env("DEVCENTER_EVIDENCE_ROOT", &evidence);
        integrations.browser(state, &mut command);
        command.env(
            "DEVCENTER_REQUIRED_INTEGRATIONS",
            serde_json::to_string(&super::integrations::required_providers(state)?)?,
        );
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
        if let Err(error) = capture(state, &format!("browser-{spec}"), &mut command) {
            if spec == "setup.spec.ts" {
                return Err(error).context("Local setup failed; dependent acceptance cannot run");
            }
            failures.push(spec);
        }
        if spec == "setup.spec.ts" {
            let project: Value = serde_json::from_slice(&fs::read(evidence.join("project.json"))?)?;
            write_private(
                &state.join("last-project.json"),
                &serde_json::to_vec(
                    &json!({"node_id":owned.container_id,"project_id":project["id"],"storage_state":evidence.join("storage-state.json"),"connector_mode":integrations.mode.as_str()}),
                )?,
            )?;
            println!(
                "Local setup available at https://devcenter.localhost:18443/connectors?tab=connections; live model acceptance follows"
            );
        }
    }
    finish_acceptance(state, &evidence, &failures)
}

fn finish_acceptance(state: &Path, evidence: &Path, failures: &[&str]) -> Result<()> {
    write_private(
        &evidence.join("checks.json"),
        &serde_json::to_vec(&json!({"failed":failures}))?,
    )?;
    acceptance_status(
        state,
        if failures.is_empty() {
            "pass"
        } else {
            "not_completed"
        },
        Some(evidence),
    )?;
    ensure!(
        failures.is_empty(),
        "Local acceptance incomplete in {}; inspect {}. Model authorization is available at https://devcenter.localhost:18443/connectors?tab=connections",
        failures.join(", "),
        evidence.display()
    );
    Ok(())
}

fn acceptance_status(state: &Path, result: &str, evidence: Option<&Path>) -> Result<()> {
    let integrations = IntegrationSelection::load(state)?;
    write_private(
        &state.join("last-acceptance.json"),
        &serde_json::to_vec(
            &json!({"result":result,"provider_mode":"live","connector_mode":integrations.mode.as_str(),"evidence":evidence,"origin":"https://devcenter.localhost:18443"}),
        )?,
    )
}

pub(super) fn invalidate_acceptance(state: &Path) -> Result<()> {
    acceptance_status(state, "not_completed", None)
}

fn record_composition(
    state: &Path,
    source: &Path,
    selected_builds: &[String],
    evidence: &Path,
) -> Result<()> {
    let integrations = IntegrationSelection::load(state)?;
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
    let mut model_endpoint_observed = false;
    for pod in pods["items"].as_array().context("running pods missing")? {
        if terminal_pod(pod) {
            continue;
        }
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
            if pod["metadata"]["labels"]["app.kubernetes.io/component"] == "agent-platform" {
                validate_live_model_arguments(&container["args"])?;
                model_endpoint_observed = true;
            }
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
    ensure!(
        model_endpoint_observed,
        "live model endpoint was not observed in Agent Platform"
    );
    for name in ["deployment.local.lock.toml", "local.json"] {
        write_private(&evidence.join(name), &fs::read(state.join(name))?)?;
    }
    write_private(
        &evidence.join("composition.json"),
        &serde_json::to_vec_pretty(
            &json!({"running":running,"source":source,"selected_builds":selected_builds,"provider_mode":"live","connector_mode":integrations.mode.as_str(),"model_endpoint":LIVE_MODEL_ENDPOINT}),
        )?,
    )?;
    Ok(())
}

fn terminal_pod(pod: &Value) -> bool {
    matches!(
        pod["status"]["phase"].as_str(),
        Some("Succeeded" | "Failed")
    )
}

fn validate_live_model_arguments(args: &Value) -> Result<()> {
    let args = args
        .as_array()
        .context("Agent Platform arguments missing")?;
    let endpoints = args
        .iter()
        .enumerate()
        .filter_map(|(index, arg)| {
            let arg = arg.as_str()?;
            if arg == "--model-endpoint-base" {
                Some(args.get(index + 1).and_then(Value::as_str).unwrap_or(""))
            } else {
                arg.strip_prefix("--model-endpoint-base=")
            }
        })
        .collect::<Vec<_>>();
    ensure!(
        endpoints == [LIVE_MODEL_ENDPOINT],
        "local acceptance requires the real Claude model endpoint; fixture or ambiguous model configuration cannot pass"
    );
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
    let owned = load_owned(state)?;
    invalidate_acceptance(state)?;
    let integrations = IntegrationSelection::load(state)?;
    super::validate_local_origin(&args.origin)?;
    ensure!(
        owned.https_port == 18443
            && args.origin.trim_end_matches('/') == "https://devcenter.localhost:18443",
        "browser acceptance must target the same owned local composition whose images are verified"
    );
    let (storage_state, project) = test_session(args, &owned.container_id)?;
    private_file(&storage_state)?;
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let output = state.join(format!("acceptance-{at}"));
    fs::create_dir(&output)?;
    acceptance_status(state, "not_completed", Some(&output))?;
    record_composition(state, &args.source, &[], &output)?;
    let mut failures = Vec::new();
    for spec in [
        "integrations.spec.ts",
        "model.spec.ts",
        "history.spec.ts",
        "lifecycle.spec.ts",
        "workspace.spec.ts",
    ] {
        let mut command = Command::new("pnpm");
        integrations.browser(state, &mut command);
        command.env(
            "DEVCENTER_REQUIRED_INTEGRATIONS",
            serde_json::to_string(&super::integrations::required_providers(state)?)?,
        );
        let result = capture(
            state,
            &format!("browser-{spec}"),
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
                .env("DEVCENTER_LOCAL_TLS_SPKI", tls_spki(state)?)
                .env("DEVCENTER_ORIGIN", &args.origin)
                .env("PROJECTS_STORAGE_STATE", &storage_state)
                .env("DEVCENTER_PROJECT_ID", &project)
                .env("DEVCENTER_PROVIDER_MODE", "live")
                .env("DEVCENTER_EVIDENCE_ROOT", &output),
        );
        if result.is_err() {
            failures.push(spec);
        }
    }
    finish_acceptance(state, &output, &failures)?;
    println!(
        "browser acceptance with live Claude passed: {}",
        output.display()
    );
    Ok(())
}

fn test_session(args: &super::Test, node_id: &str) -> Result<(PathBuf, String)> {
    if let (Some(storage), Some(project)) = (&args.storage_state, &args.project) {
        return Ok((storage.clone(), project.clone()));
    }
    let record: Value = serde_json::from_slice(
        &fs::read(args.target.state.join("last-project.json"))
            .context("run local up first, or provide --storage-state and --project")?,
    )?;
    let integrations = IntegrationSelection::load(&args.target.state)?;
    ensure!(
        record["connector_mode"].as_str().unwrap_or("fixture") == integrations.mode.as_str(),
        "saved project uses another Connector mode; run local up setup for the selected providers"
    );
    ensure!(
        record["node_id"] == node_id,
        "saved setup belongs to a different node; run local up again"
    );
    let storage = args
        .storage_state
        .clone()
        .or_else(|| record["storage_state"].as_str().map(PathBuf::from))
        .context("saved setup predates session reuse; run local up or provide --storage-state")?;
    let project = args
        .project
        .clone()
        .or_else(|| record["project_id"].as_str().map(str::to_owned))
        .context("saved local project missing")?;
    Ok((storage, project))
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
    fn automatic_session_reuse_refuses_a_replaced_node() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        write_private(&state.join("last-project.json"), &serde_json::to_vec(&json!({
            "node_id":"original-node", "project_id":"project-local", "storage_state":"private-session.json"
        })).unwrap()).unwrap();
        let args = super::super::Test {
            target: Target {
                state: state.to_path_buf(),
            },
            source: state.to_path_buf(),
            origin: "https://devcenter.localhost:18443".into(),
            storage_state: None,
            project: None,
        };
        assert_eq!(
            test_session(&args, "original-node").unwrap(),
            (
                PathBuf::from("private-session.json"),
                "project-local".into()
            )
        );
        assert!(test_session(&args, "replacement-node").is_err());
    }

    #[test]
    fn acceptance_refuses_fixture_ambiguous_and_missing_model_endpoints() {
        for args in [
            json!(["serve"]),
            json!([
                "serve",
                "--model-endpoint-base",
                "http://provider.localhost/v1"
            ]),
            json!([
                "--model-endpoint-base",
                LIVE_MODEL_ENDPOINT,
                "--model-endpoint-base=http://provider.localhost/v1"
            ]),
        ] {
            assert!(validate_live_model_arguments(&args).is_err());
        }
        assert!(
            validate_live_model_arguments(&json!([
                "serve",
                "--model-endpoint-base",
                LIVE_MODEL_ENDPOINT
            ]))
            .is_ok()
        );
        assert!(
            validate_live_model_arguments(&json!([format!(
                "--model-endpoint-base={LIVE_MODEL_ENDPOINT}"
            )]))
            .is_ok()
        );
    }

    #[test]
    fn historical_pods_are_excluded_but_pending_workloads_still_require_readiness() {
        for phase in ["Succeeded", "Failed"] {
            assert!(terminal_pod(&json!({"status":{"phase":phase}})));
        }
        for phase in ["Running", "Pending", "Unknown"] {
            assert!(!terminal_pod(&json!({"status":{"phase":phase}})));
        }
    }

    #[test]
    fn new_attempt_invalidates_an_older_pass() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        acceptance_status(state, "pass", Some(Path::new("old-evidence"))).unwrap();
        acceptance_status(state, "not_completed", None).unwrap();
        let status: Value =
            serde_json::from_slice(&fs::read(state.join("last-acceptance.json")).unwrap()).unwrap();
        assert_eq!(status["result"], "not_completed");
        assert_eq!(status["provider_mode"], "live");
        assert!(status["evidence"].is_null());
    }

    #[test]
    fn independent_successes_do_not_hide_a_failed_acceptance_suite() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        acceptance_status(state, "pass", Some(Path::new("old-evidence"))).unwrap();
        assert!(finish_acceptance(state, state, &["model.spec.ts", "workspace.spec.ts"]).is_err());
        let status: Value =
            serde_json::from_slice(&fs::read(state.join("last-acceptance.json")).unwrap()).unwrap();
        let checks: Value =
            serde_json::from_slice(&fs::read(state.join("checks.json")).unwrap()).unwrap();
        assert_eq!(status["result"], "not_completed");
        assert_eq!(
            checks["failed"],
            json!(["model.spec.ts", "workspace.spec.ts"])
        );
    }

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
        select_candidate(state, "server", "devcenter").unwrap();
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
