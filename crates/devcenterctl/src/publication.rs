//! Publication policy over the existing ESS build outputs.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    process::Command,
};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Provenance {
    pub version: String,
    pub source_commit: String,
    pub digest: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub schema: u32,
    pub version: String,
    pub source_commit: String,
    pub artifacts: BTreeMap<String, String>,
    #[serde(default)]
    pub provenance: BTreeMap<String, Provenance>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Plan {
    pub version: String,
    pub source_commit: String,
    pub selected: BTreeSet<String>,
    pub reused: BTreeMap<String, Provenance>,
    pub outputs: BTreeMap<String, Output>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Output {
    pub release_unit: String,
    pub kind: String,
}

pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(
        &std::fs::read(path).with_context(|| format!("read {}", path.display()))?,
    )
    .with_context(|| format!("decode {}", path.display()))
}

fn git(root: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    ensure!(
        output.status.success(),
        "Git history unavailable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output.stdout)
}

pub fn outputs(root: &Path) -> Result<BTreeMap<String, Output>> {
    let ir: Value = read_json(&root.join("generated/ess/build.json"))?;
    ensure!(ir["format"] == "ess-build-ir/1", "unsupported ESS build IR");
    let mut outputs = BTreeMap::new();
    for output in ir["outputs"]
        .as_object()
        .context("ESS outputs missing")?
        .values()
    {
        let name = output["name"].as_str().context("ESS output name missing")?;
        artifact_key(name)?;
        outputs.insert(name.to_owned(), serde_json::from_value(output.clone())?);
    }
    ensure!(
        outputs.len() == 4,
        "expected the four published ESS outputs"
    );
    Ok(outputs)
}

fn artifact_key(output: &str) -> Result<&str> {
    // Existing schema-1 consumer names; release identities are read from ESS.
    match output {
        "server" => Ok("devcenter"),
        "deployment-cli" => Ok("devcenterctl"),
        "connectors" => Ok("devcenter_connectors"),
        "chart" => Ok("chart"),
        _ => bail!("unsupported ESS publication output {output}"),
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id.len() <= 96
        && id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c))
}

fn chart_version(version: &str) -> bool {
    let (core, prerelease) = version
        .split_once('-')
        .map_or((version, None), |(core, pre)| (core, Some(pre)));
    let numeric = |part: &str| {
        !part.is_empty()
            && part.bytes().all(|c| c.is_ascii_digit())
            && (part.len() == 1 || !part.starts_with('0'))
    };
    let parts = core.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.into_iter().all(numeric)
        && prerelease.is_none_or(|pre| {
            pre.split('.').all(|part| {
                !part.is_empty()
                    && part.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
                    && (!part.bytes().all(|c| c.is_ascii_digit()) || numeric(part))
            })
        })
}
fn valid_sha(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|c| c.is_ascii_hexdigit())
}
fn validate_provenance(p: &Provenance) -> Result<()> {
    ensure!(
        valid_id(&p.version) && valid_sha(&p.source_commit),
        "invalid artifact provenance"
    );
    ensure!(
        p.digest.starts_with("sha256:")
            && p.digest.len() == 71
            && p.digest[7..].bytes().all(|c| c.is_ascii_hexdigit()),
        "invalid artifact digest"
    );
    Ok(())
}

fn provenance(
    manifest: &Manifest,
    outputs: &BTreeMap<String, Output>,
) -> Result<BTreeMap<String, Provenance>> {
    ensure!(
        manifest.schema == 1 && valid_id(&manifest.version) && valid_sha(&manifest.source_commit),
        "invalid release manifest"
    );
    ensure!(
        manifest.artifacts.len() == outputs.len(),
        "incomplete publication manifest"
    );
    ensure!(
        manifest.provenance.is_empty() || manifest.provenance.len() == outputs.len(),
        "incomplete publication provenance"
    );
    outputs
        .keys()
        .map(|name| {
            let digest = manifest
                .artifacts
                .get(artifact_key(name)?)
                .context("missing artifact digest")?;
            let p = if manifest.provenance.is_empty() {
                Provenance {
                    version: manifest.version.clone(),
                    source_commit: manifest.source_commit.clone(),
                    digest: digest.clone(),
                }
            } else {
                manifest
                    .provenance
                    .get(name)
                    .context("missing artifact provenance")?
                    .clone()
            };
            validate_provenance(&p)?;
            ensure!(
                &p.digest == digest,
                "artifact digest and provenance disagree"
            );
            Ok((name.clone(), p))
        })
        .collect()
}

/// Classify source paths conservatively. Shared recipe changes select all image consumers.
pub fn affected(path: &str) -> BTreeSet<String> {
    let images = ["server", "connectors", "deployment-cli"];
    let units: &[&str] = if path.starts_with("deploy/charts/") {
        &["chart"]
    } else if path.starts_with("frontend/") || path == "openapi.json" {
        &["server"]
    } else if path.starts_with("crates/devcenter-connectors/") {
        &["connectors"]
    } else if path.starts_with("crates/devcenterctl/") {
        &["deployment-cli"]
    } else if path.starts_with("crates/") {
        &["server"]
    } else if matches!(path, "Cargo.toml" | "Cargo.lock" | "rust-toolchain.toml")
        || path.starts_with(".cargo/")
    {
        &["server", "deployment-cli"]
    } else if matches!(path, "ess/build.yaml" | "generated/ess/build.json") {
        &["server", "connectors", "deployment-cli", "chart"]
    } else if path == "generated/ess/build.mmd" {
        &[]
    } else if path.starts_with("Dockerfile")
        || path == "docker-bake.hcl"
        || path == ".github/workflows/release.yml"
    {
        &images
    } else if [
        ".engineering/",
        "docs/",
        "changes/",
        ".github/",
        "ci/",
        "ess/system/",
    ]
    .iter()
    .any(|p| path.starts_with(p))
        || matches!(
            path,
            ".dockerignore"
                | ".gitignore"
                | "AGENTS.md"
                | "CHANGELOG.md"
                | "LICENSE"
                | "README.md"
                | "b10x.docs.yaml"
        )
    {
        &[]
    } else {
        &images
    };
    units.iter().map(|s| (*s).to_owned()).collect()
}

fn normalized(path: &str, bytes: &[u8]) -> Result<Value> {
    if path.ends_with("Cargo.toml") || path.ends_with("Cargo.lock") {
        let value: toml::Value = toml::from_str(std::str::from_utf8(bytes)?)?;
        let mut value = serde_json::to_value(value)?;
        if path.ends_with("Cargo.toml") {
            for pointer in ["/workspace/package", "/package"] {
                if let Some(package) = value.pointer_mut(pointer).and_then(Value::as_object_mut) {
                    package.remove("version");
                }
            }
        } else if let Some(packages) = value["package"].as_array_mut() {
            for package in packages {
                // Only this product's local package versions are release bookkeeping.
                if package.get("source").is_none()
                    && package["name"]
                        .as_str()
                        .is_some_and(|n| n.starts_with("devcenter"))
                {
                    package
                        .as_object_mut()
                        .context("invalid lock package")?
                        .remove("version");
                }
            }
        }
        return Ok(value);
    }
    if matches!(path, "frontend/package.json" | "openapi.json") {
        let mut value: Value = serde_json::from_slice(bytes)?;
        if path == "openapi.json" {
            value["info"]
                .as_object_mut()
                .context("OpenAPI info missing")?
                .remove("version");
        } else {
            value
                .as_object_mut()
                .context("package object missing")?
                .remove("version");
        }
        return Ok(value);
    }
    if path == "deploy/charts/devcenter/Chart.yaml" {
        let mut value: Value = serde_yaml::from_slice(bytes)?;
        let object = value.as_object_mut().context("chart object missing")?;
        object.remove("version");
        object.remove("appVersion");
        return Ok(value);
    }
    Ok(Value::Array(
        bytes.iter().map(|b| Value::from(*b)).collect(),
    ))
}

fn changed(root: &Path, base: &str, head: &str, unit: &str) -> Result<bool> {
    git(root, &["merge-base", "--is-ancestor", base, head])?;
    let paths = git(
        root,
        &["diff", "--name-only", "--no-renames", "-z", base, head],
    )?;
    let recipe_changed = paths
        .split(|b| *b == 0)
        .any(|p| p == b"generated/ess/build.json");
    for path in paths.split(|b| *b == 0).filter(|p| !p.is_empty()) {
        let path = std::str::from_utf8(path)?;
        if !affected(path).contains(unit) {
            continue;
        }
        if recipe_changed && matches!(path, "Dockerfile.ess" | "docker-bake.hcl") {
            // These are validated projections of this graph. Their whole-file diff
            // must not turn one changed recipe into rebuilding every image.
            if recipe(root, base, "generated/ess/build.json", unit)?
                == recipe(root, head, "generated/ess/build.json", unit)?
            {
                continue;
            }
            return Ok(true);
        }
        if matches!(path, "ess/build.yaml" | "generated/ess/build.json") {
            if recipe(root, base, path, unit)? == recipe(root, head, path, unit)? {
                continue;
            }
            return Ok(true);
        }
        // Added/deleted paths always affect their consumers. Existing version-bearing files
        // compare parsed content after removing only product-owned release versions.
        let old = git(root, &["show", &format!("{base}:{path}")]);
        let new = git(root, &["show", &format!("{head}:{path}")]);
        if let (Ok(old), Ok(new)) = (old, new)
            && normalized(path, &old)? == normalized(path, &new)?
        {
            continue;
        }
        return Ok(true);
    }
    Ok(false)
}

fn recipe(root: &Path, revision: &str, path: &str, unit: &str) -> Result<Value> {
    fn named(value: &Value, key: &str) -> Result<BTreeMap<String, Value>> {
        if let Some(map) = value.as_object() {
            return Ok(map.iter().map(|(k, v)| (k.clone(), v.clone())).collect());
        }
        value
            .as_array()
            .context("invalid ESS graph collection")?
            .iter()
            .map(|v| {
                Ok((
                    v[key]
                        .as_str()
                        .context("missing ESS graph identity")?
                        .to_owned(),
                    v.clone(),
                ))
            })
            .collect()
    }
    let bytes = git(root, &["show", &format!("{revision}:{path}")])?;
    let graph: Value = if path == "ess/build.yaml" {
        serde_yaml::from_slice(&bytes)?
    } else {
        serde_json::from_slice(&bytes)?
    };
    let nodes = named(&graph["nodes"], "id")?;
    let outputs = named(&graph["outputs"], "name")?;
    let output = outputs.get(unit).context("missing ESS output")?;
    let mut pending = vec![
        output["node"]
            .as_str()
            .context("missing ESS output node")?
            .to_owned(),
    ];
    let mut reachable = BTreeMap::new();
    while let Some(id) = pending.pop() {
        if reachable.contains_key(&id) {
            continue;
        }
        let node = nodes.get(&id).context("missing reachable ESS build node")?;
        for edge in ["base", "from", "rootfs"] {
            if let Some(parent) = node[edge].as_str() {
                pending.push(parent.to_owned());
            }
        }
        reachable.insert(id, node.clone());
    }
    Ok(
        serde_json::json!({"output":output, "nodes":reachable, "platforms":graph["platforms"], "secrets":graph["secrets"]}),
    )
}

pub fn plan(
    root: &Path,
    history: &[Manifest],
    version: &str,
    unit: &str,
    bootstrap: bool,
) -> Result<Plan> {
    ensure!(valid_id(version), "invalid release identifier");
    let outputs = outputs(root)?;
    ensure!(
        unit == "auto" || outputs.contains_key(unit),
        "unknown publication unit"
    );
    let source_commit = String::from_utf8(git(root, &["rev-parse", "HEAD"])?)?
        .trim()
        .to_owned();
    ensure!(
        !history.is_empty() || bootstrap,
        "no successful publication baseline; explicit --bootstrap required"
    );
    ensure!(
        !bootstrap || history.is_empty(),
        "bootstrap is only valid with empty verified history"
    );
    ensure!(
        !bootstrap || unit == "auto",
        "bootstrap must publish all outputs"
    );
    let mut latest = BTreeMap::new();
    let mut existing = None;
    for manifest in history {
        let metadata = provenance(manifest, &outputs)?;
        git(
            root,
            &[
                "merge-base",
                "--is-ancestor",
                &manifest.source_commit,
                &source_commit,
            ],
        )?;
        for p in metadata.values() {
            git(
                root,
                &[
                    "merge-base",
                    "--is-ancestor",
                    &p.source_commit,
                    &manifest.source_commit,
                ],
            )?;
        }
        if manifest.version == version {
            ensure!(
                manifest.source_commit == source_commit,
                "immutable release identifier already belongs to another commit"
            );
            existing = Some(metadata.clone());
        }
        if latest.is_empty() {
            latest = metadata;
        }
    }
    if let Some(reused) = existing {
        return Ok(Plan {
            version: version.into(),
            source_commit,
            selected: BTreeSet::new(),
            reused,
            outputs,
        });
    }
    let mut selected = BTreeSet::new();
    for name in outputs.keys() {
        if unit != "auto" && unit != name {
            continue;
        }
        if let Some(base) = latest.get(name)
            && !changed(root, &base.source_commit, &source_commit, name)?
        {
            continue;
        }
        selected.insert(name.clone());
    }
    let reused = latest
        .into_iter()
        .filter(|(name, _)| !selected.contains(name))
        .collect();
    ensure!(
        !selected.contains("chart") || chart_version(version),
        "chart publication requires a SemVer identifier"
    );
    Ok(Plan {
        version: version.into(),
        source_commit,
        selected,
        reused,
        outputs,
    })
}

pub fn complete(plan: Plan, receipts: BTreeMap<String, Provenance>) -> Result<Manifest> {
    ensure!(
        receipts.keys().cloned().collect::<BTreeSet<_>>() == plan.selected,
        "incomplete or unexpected publication receipts"
    );
    let mut metadata = plan.reused;
    for (name, receipt) in receipts {
        validate_provenance(&receipt)?;
        ensure!(
            receipt.version == plan.version && receipt.source_commit == plan.source_commit,
            "publication receipt does not match candidate"
        );
        metadata.insert(name, receipt);
    }
    let artifacts = metadata
        .iter()
        .map(|(name, p)| Ok((artifact_key(name)?.into(), p.digest.clone())))
        .collect::<Result<_>>()?;
    let manifest = Manifest {
        schema: 1,
        version: plan.version,
        source_commit: plan.source_commit,
        artifacts,
        provenance: metadata,
    };
    provenance(&manifest, &plan.outputs)?;
    Ok(manifest)
}
