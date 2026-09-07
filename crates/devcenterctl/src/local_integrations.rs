//! Retained, explicit provider selection for local development.
use super::{private_file, write_private};
use anyhow::{Context, Result, ensure};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub(super) enum Mode {
    #[default]
    Fixture,
    Live,
}

impl Mode {
    pub(super) const fn as_str(self) -> &'static str {
        match self {
            Self::Fixture => "fixture",
            Self::Live => "live",
        }
    }
}

#[derive(Debug, Clone, Default, Args)]
pub(super) struct Options {
    /// Retain real provider configuration from the private baseline, or use the Git fixture.
    /// Omission reuses this installation's previous selection.
    #[arg(long)]
    connector_mode: Option<Mode>,
    /// Owner-only generic credential manifest. Values stay in separately referenced private files.
    #[arg(long)]
    provisioning_file: Option<PathBuf>,
    /// Real provider repository reference used by workspace acceptance.
    #[arg(long)]
    repository_ref: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Selection {
    pub mode: Mode,
    pub provisioning_file: Option<PathBuf>,
    pub repository_ref: Option<String>,
}

impl Selection {
    pub(super) fn load(state: &Path) -> Result<Self> {
        let path = state.join("integrations.local.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        serde_json::from_slice(&fs::read(path)?).context("invalid retained integration selection")
    }

    pub(super) fn select(state: &Path, options: &Options) -> Result<Self> {
        let mut selected = Self::load(state)?;
        if let Some(mode) = options.connector_mode {
            if mode != selected.mode {
                selected.provisioning_file = None;
                selected.repository_ref = None;
            }
            selected.mode = mode;
        }
        if let Some(path) = &options.provisioning_file {
            private_file(path)?;
            selected.provisioning_file = Some(fs::canonicalize(path)?);
        }
        if let Some(repository) = &options.repository_ref {
            ensure!(
                !repository.trim().is_empty(),
                "repository reference must not be empty"
            );
            selected.repository_ref = Some(repository.clone());
        }
        Ok(selected)
    }

    pub(super) fn save(&self, state: &Path) -> Result<()> {
        write_private(
            &state.join("integrations.local.json"),
            &serde_json::to_vec(self)?,
        )
    }

    pub(super) fn browser(&self, state: &Path, command: &mut Command) {
        command.env("DEVCENTER_CONNECTOR_MODE", self.mode.as_str());
        if self.mode == Mode::Fixture {
            command.env(
                "DEVCENTER_PROVISIONING_FILE",
                state.join("fixture-provisioning.json"),
            );
            command.env("DEVCENTER_REPOSITORY_REF", "1");
        } else {
            command.env_remove("DEVCENTER_PROVISIONING_FILE");
            command.env_remove("DEVCENTER_REPOSITORY_REF");
            if let Some(path) = &self.provisioning_file {
                command.env("DEVCENTER_PROVISIONING_FILE", path);
            }
            if let Some(repository) = &self.repository_ref {
                command.env("DEVCENTER_REPOSITORY_REF", repository);
            }
        }
    }
}

/// Acceptance targets come from the composed deployment, never an ambient fixture selection.
pub(super) fn required_providers(state: &Path) -> Result<Vec<String>> {
    if Selection::load(state)?.mode == Mode::Fixture {
        return Ok(Vec::new());
    }
    let values: serde_json::Value =
        serde_yaml::from_slice(&fs::read(state.join("values.local.yaml"))?)?;
    let hosted = values["components"]["connectors"]["configFiles"]["hosted.toml"]
        .as_str()
        .context("composed hosted Connector configuration missing")?;
    let config: toml::Value = toml::from_str(hosted)?;
    let mut providers = std::collections::BTreeSet::new();
    for provider in ["gitlab", "slack", "grafana"] {
        if config
            .get(provider)
            .is_some_and(|value| value.get("enabled").and_then(toml::Value::as_bool) != Some(false))
        {
            providers.insert(provider.to_owned());
        }
    }
    if let Some(bindings) = config
        .get("catalog")
        .and_then(|value| value.get("bindings"))
        .and_then(toml::Value::as_table)
    {
        providers.extend(bindings.keys().cloned());
    }
    Ok(providers.into_iter().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn live_acceptance_requires_native_and_deployment_bound_providers() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        assert!(required_providers(state).unwrap().is_empty());
        Selection {
            mode: Mode::Live,
            ..Selection::default()
        }
        .save(state)
        .unwrap();
        assert!(required_providers(state).is_err());
        let values = serde_json::json!({"components":{"connectors":{"configFiles":{
            "hosted.toml": "[gitlab]\norigin='https://forge.example.test'\n[slack]\nenabled=false\n[catalog.bindings.grafana.endpoints]\norigin='https://metrics.example.test'"
        }}}});
        write_private(
            &state.join("values.local.yaml"),
            serde_yaml::to_string(&values).unwrap().as_bytes(),
        )
        .unwrap();
        assert_eq!(
            required_providers(state).unwrap(),
            vec!["gitlab", "grafana"]
        );
    }

    #[test]
    fn live_selection_is_retained_and_fixture_manifest_is_never_inherited() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        let manifest = state.join("provision.json");
        write_private(&manifest, b"[]").unwrap();
        Selection::select(
            state,
            &Options {
                connector_mode: Some(Mode::Live),
                provisioning_file: Some(manifest.clone()),
                repository_ref: Some("repository-test".into()),
            },
        )
        .unwrap()
        .save(state)
        .unwrap();
        let selection = Selection::select(state, &Options::default()).unwrap();
        assert_eq!(selection.mode, Mode::Live);
        assert_eq!(selection.repository_ref.as_deref(), Some("repository-test"));
        assert_eq!(selection.provisioning_file, Some(manifest));
        let mut command = Command::new("browser");
        selection.browser(state, &mut command);
        assert_eq!(
            command
                .get_envs()
                .find(|(key, _)| *key == "DEVCENTER_CONNECTOR_MODE")
                .unwrap()
                .1
                .unwrap(),
            "live"
        );
        let fixture = Selection::select(
            state,
            &Options {
                connector_mode: Some(Mode::Fixture),
                ..Options::default()
            },
        )
        .unwrap();
        assert!(fixture.provisioning_file.is_none());
        assert!(fixture.repository_ref.is_none());
    }

    #[test]
    fn missing_or_public_manifests_fail_and_live_mode_clears_inherited_fixture_variables() {
        let directory = tempfile::tempdir().unwrap();
        let state = directory.path();
        let manifest = state.join("public.json");
        assert!(
            Selection::select(
                state,
                &Options {
                    provisioning_file: Some(manifest.clone()),
                    ..Options::default()
                }
            )
            .is_err()
        );
        fs::write(&manifest, b"[]").unwrap();
        fs::set_permissions(&manifest, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(
            Selection::select(
                state,
                &Options {
                    provisioning_file: Some(manifest),
                    ..Options::default()
                }
            )
            .is_err()
        );
        let mut command = Command::new("browser");
        command
            .env("DEVCENTER_PROVISIONING_FILE", "fixture.json")
            .env("DEVCENTER_REPOSITORY_REF", "1");
        Selection {
            mode: Mode::Live,
            ..Selection::default()
        }
        .browser(state, &mut command);
        for key in ["DEVCENTER_PROVISIONING_FILE", "DEVCENTER_REPOSITORY_REF"] {
            assert!(
                command
                    .get_envs()
                    .find(|(name, _)| *name == key)
                    .unwrap()
                    .1
                    .is_none()
            );
        }
    }
}
