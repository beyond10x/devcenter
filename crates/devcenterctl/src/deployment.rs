//! Deployment-lock and rendered-workload validation.

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_yaml::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One immutable artifact admitted by a deployment lock.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedArtifact {
    /// Registry or OCI repository without a tag or digest.
    pub reference: String,
    /// Human-readable released version.
    pub version: String,
    /// Immutable OCI digest.
    pub digest: String,
}

/// Complete immutable input set for one environment deployment.
#[derive(Debug, Deserialize)]
pub struct DeploymentLock {
    /// Lock schema version.
    pub schema: u32,
    /// Chart selected for the deployment.
    pub chart: LockedArtifact,
    /// Images admitted in rendered Kubernetes workloads.
    pub images: BTreeMap<String, LockedArtifact>,
}

impl DeploymentLock {
    /// Read and validate one deployment lock.
    pub fn read(path: &Path) -> Result<Self> {
        let body = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read deployment lock {}", path.display()))?;
        let lock: Self = toml::from_str(&body).context("deployment lock is not valid TOML")?;
        if lock.schema != 1 {
            bail!("unsupported deployment lock schema {}", lock.schema);
        }
        validate_artifact("chart", &lock.chart)?;
        if lock.images.is_empty() {
            bail!("deployment lock contains no images");
        }
        for (name, artifact) in &lock.images {
            validate_artifact(&format!("image {name}"), artifact)?;
        }
        Ok(lock)
    }

    /// Require the requested chart reference and version to match the lock.
    pub fn validate_chart(&self, reference: &str, version: Option<&str>) -> Result<()> {
        if self.chart.reference != reference {
            bail!(
                "chart reference `{reference}` does not match deployment lock `{}`",
                self.chart.reference
            );
        }
        let version = version.context("chart version is required for a locked deployment")?;
        if self.chart.version != version {
            bail!(
                "chart version `{version}` does not match deployment lock `{}`",
                self.chart.version
            );
        }
        Ok(())
    }
}

/// Validate every rendered workload image and required component against a lock.
pub fn validate_rendered(
    lock: &DeploymentLock,
    rendered: &str,
    required_components: &[String],
) -> Result<()> {
    let admitted = lock
        .images
        .iter()
        .map(|(name, artifact)| {
            (
                format!(
                    "{}@{}",
                    canonical_image_reference(&artifact.reference),
                    artifact.digest
                ),
                name.as_str(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let required = required_components.iter().cloned().collect::<BTreeSet<_>>();
    if required.len() != required_components.len() {
        bail!("required component list contains duplicates");
    }

    let mut observed_components = BTreeSet::new();
    let mut observed_images = 0usize;
    for document in serde_yaml::Deserializer::from_str(rendered) {
        let value = Value::deserialize(document).context("rendered chart contains invalid YAML")?;
        let Some(kind) = string_at(&value, &["kind"]) else {
            continue;
        };
        let pod_template = match kind {
            "Deployment" | "StatefulSet" | "DaemonSet" | "Job" => {
                value_at(&value, &["spec", "template"])
            }
            "CronJob" => value_at(&value, &["spec", "jobTemplate", "spec", "template"]),
            _ => None,
        };
        let Some(template) = pod_template else {
            continue;
        };
        if let Some(component) = string_at(
            template,
            &["metadata", "labels", "app.kubernetes.io/component"],
        ) {
            observed_components.insert(component.to_owned());
        }
        for group in ["initContainers", "containers"] {
            let Some(containers) =
                value_at(template, &["spec", group]).and_then(Value::as_sequence)
            else {
                continue;
            };
            for container in containers {
                let name = string_at(container, &["name"]).unwrap_or("unnamed");
                let image = string_at(container, &["image"])
                    .with_context(|| format!("{kind} container `{name}` has no image"))?;
                observed_images += 1;
                validate_rendered_image(image, name, &admitted)?;
            }
        }
    }
    if observed_images == 0 {
        bail!("rendered chart contains no workload images");
    }
    let missing = required
        .difference(&observed_components)
        .cloned()
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!(
            "required components are disabled or absent: {}",
            missing.join(", ")
        );
    }
    Ok(())
}

fn validate_artifact(name: &str, artifact: &LockedArtifact) -> Result<()> {
    if artifact.reference.trim().is_empty()
        || artifact.reference.contains('@')
        || artifact
            .reference
            .rsplit('/')
            .next()
            .is_some_and(|part| part.contains(':'))
    {
        bail!("{name} reference must not contain a tag or digest");
    }
    if artifact.version.trim().is_empty() {
        bail!("{name} version is empty");
    }
    if !valid_digest(&artifact.digest) {
        bail!("{name} digest is not a sha256 digest");
    }
    Ok(())
}

fn validate_rendered_image(
    image: &str,
    container: &str,
    admitted: &BTreeMap<String, &str>,
) -> Result<()> {
    let Some((reference, digest)) = image.rsplit_once('@') else {
        bail!("container `{container}` uses mutable image `{image}`");
    };
    if !valid_digest(digest) {
        bail!("container `{container}` uses invalid digest in `{image}`");
    }
    let canonical = format!("{}@{digest}", canonical_image_reference(reference));
    if !admitted.contains_key(&canonical) {
        bail!("container `{container}` image `{image}` is absent from deployment lock");
    }
    Ok(())
}

fn canonical_image_reference(reference: &str) -> String {
    let first = reference.split('/').next().unwrap_or_default();
    if reference.contains('/')
        && (first.contains('.') || first.contains(':') || first == "localhost")
    {
        reference.to_owned()
    } else if reference.contains('/') {
        format!("docker.io/{reference}")
    } else {
        format!("docker.io/library/{reference}")
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    path.iter().try_fold(value, |current, segment| {
        current
            .as_mapping()?
            .get(Value::String((*segment).to_owned()))
    })
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    value_at(value, path)?.as_str()
}

#[cfg(test)]
mod tests {
    use super::{DeploymentLock, LockedArtifact, validate_rendered};
    use std::collections::BTreeMap;

    const DIGEST: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn lock() -> DeploymentLock {
        DeploymentLock {
            schema: 1,
            chart: LockedArtifact {
                reference: "oci://registry.example/charts/devcenter".to_owned(),
                version: "1.2.3".to_owned(),
                digest: DIGEST.to_owned(),
            },
            images: BTreeMap::from([(
                "workspace".to_owned(),
                LockedArtifact {
                    reference: "registry.example/workspace".to_owned(),
                    version: "1.0.0".to_owned(),
                    digest: DIGEST.to_owned(),
                },
            )]),
        }
    }

    #[test]
    fn every_rendered_image_and_required_component_must_be_locked() {
        let rendered = format!(
            "kind: Deployment\nmetadata:\n  name: workspace\nspec:\n  template:\n    metadata:\n      labels:\n        app.kubernetes.io/component: workspace\n    spec:\n      containers:\n        - name: workspace\n          image: registry.example/workspace@{DIGEST}\n"
        );
        validate_rendered(&lock(), &rendered, &["workspace".to_owned()]).unwrap();
    }

    #[test]
    fn mutable_or_unlocked_images_are_refused() {
        let rendered = "kind: Deployment\nspec:\n  template:\n    metadata:\n      labels:\n        app.kubernetes.io/component: workspace\n    spec:\n      containers:\n        - name: workspace\n          image: registry.example/workspace:latest\n";
        let error = validate_rendered(&lock(), rendered, &[]).unwrap_err();
        assert!(error.to_string().contains("mutable image"));
    }

    #[test]
    fn disabled_required_components_are_refused() {
        let rendered = format!(
            "kind: Deployment\nspec:\n  template:\n    metadata:\n      labels:\n        app.kubernetes.io/component: devcenter\n    spec:\n      containers:\n        - name: workspace\n          image: registry.example/workspace@{DIGEST}\n"
        );
        let error = validate_rendered(&lock(), &rendered, &["workspace".to_owned()]).unwrap_err();
        assert!(error.to_string().contains("workspace"));
    }

    #[test]
    fn docker_hub_shorthand_matches_the_canonical_lock_reference() {
        let mut lock = lock();
        lock.images.insert(
            "postgres".to_owned(),
            LockedArtifact {
                reference: "docker.io/library/postgres".to_owned(),
                version: "18".to_owned(),
                digest: DIGEST.to_owned(),
            },
        );
        let rendered = format!(
            "kind: StatefulSet\nspec:\n  template:\n    metadata:\n      labels:\n        app.kubernetes.io/component: database\n    spec:\n      containers:\n        - name: postgres\n          image: postgres@{DIGEST}\n"
        );
        validate_rendered(&lock, &rendered, &[]).unwrap();
    }
}
