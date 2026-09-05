use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

fn git(root: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}
fn put(root: &Path, path: &str, body: &str) {
    fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    fs::write(root.join(path), body).unwrap();
}
fn commit(root: &Path) -> String {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn fixture() -> (tempfile::TempDir, String, Value) {
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "-q"]);
    put(
        tmp.path(),
        "generated/ess/build.json",
        include_str!("../../../generated/ess/build.json"),
    );
    put(
        tmp.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"1.0.0\"\n[workspace.dependencies]\nanyhow = \"1\"\n",
    );
    let sha = commit(tmp.path());
    let manifest = json!({"schema":1,"version":"1.0.0","source_commit":sha,"artifacts":{
        "devcenter":format!("sha256:{}", "a".repeat(64)),
        "devcenterctl":format!("sha256:{}", "b".repeat(64)),
        "devcenter_connectors":format!("sha256:{}", "c".repeat(64)),
        "chart":format!("sha256:{}", "d".repeat(64))}});
    (tmp, sha, manifest)
}
fn plan(root: &Path, history: &Value, selection: &str, bootstrap: bool) -> Result<Value, String> {
    let history_file = root.join("history.json");
    fs::write(&history_file, history.to_string()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_devcenterctl"));
    command.current_dir(root).args([
        "release",
        "plan",
        "--history",
        "history.json",
        "--version",
        "2.0.0",
        "--unit",
        selection,
    ]);
    if bootstrap {
        command.arg("--bootstrap");
    }
    let output = command.output().unwrap();
    if output.status.success() {
        Ok(serde_json::from_slice(&output.stdout).unwrap())
    } else {
        Err(String::from_utf8(output.stderr).unwrap())
    }
}
#[test]
fn independent_surface_selection() {
    for (path, expected) in [
        ("frontend/src/App.vue", "server"),
        ("crates/devcenterctl/src/main.rs", "deployment-cli"),
        ("crates/devcenter-connectors/src/main.rs", "connectors"),
        ("deploy/charts/devcenter/templates/a.yaml", "chart"),
    ] {
        let (tmp, _, baseline) = fixture();
        put(tmp.path(), path, "changed");
        commit(tmp.path());
        assert_eq!(
            plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
            json!([expected])
        );
    }
}
#[test]
fn documentation_is_noop() {
    let (tmp, _, baseline) = fixture();
    put(tmp.path(), "docs/a.md", "docs");
    commit(tmp.path());
    assert_eq!(
        plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
        json!([])
    );
}
#[test]
fn own_release_version_does_not_rebuild() {
    let (tmp, _, baseline) = fixture();
    put(
        tmp.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"2.0.0\"\n[workspace.dependencies]\nanyhow = \"1\"\n",
    );
    commit(tmp.path());
    assert_eq!(
        plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
        json!([])
    );
}
#[test]
fn real_dependency_change_selects_dependents() {
    let (tmp, _, baseline) = fixture();
    put(
        tmp.path(),
        "Cargo.toml",
        "[workspace.package]\nversion = \"2.0.0\"\n[workspace.dependencies]\nanyhow = \"2\"\n",
    );
    commit(tmp.path());
    assert_eq!(
        plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
        json!(["deployment-cli", "server"])
    );
}
#[test]
fn absent_history_requires_explicit_bootstrap() {
    let (tmp, _, _) = fixture();
    assert!(
        plan(tmp.path(), &json!([]), "auto", false)
            .unwrap_err()
            .contains("bootstrap")
    );
    assert_eq!(
        plan(tmp.path(), &json!([]), "auto", true).unwrap()["selected"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
}
#[test]
fn malformed_or_incomplete_history_refuses() {
    let (tmp, _, baseline) = fixture();
    let mut broken = baseline.clone();
    broken["artifacts"]["chart"] = json!("bad");
    assert!(
        plan(tmp.path(), &json!([broken]), "auto", false)
            .unwrap_err()
            .contains("invalid artifact digest")
    );
    let mut incomplete = baseline;
    incomplete["artifacts"]
        .as_object_mut()
        .unwrap()
        .remove("chart");
    assert!(
        plan(tmp.path(), &json!([incomplete]), "auto", false)
            .unwrap_err()
            .contains("incomplete publication")
    );
}
#[test]
fn missing_source_history_refuses() {
    let (tmp, _, mut baseline) = fixture();
    baseline["source_commit"] = json!("f".repeat(40));
    assert!(
        plan(tmp.path(), &json!([baseline]), "auto", false)
            .unwrap_err()
            .contains("Git history unavailable")
    );
}
#[test]
fn explicit_unit_does_not_mask_staggered_changes() {
    let (tmp, old_sha, baseline) = fixture();
    put(tmp.path(), "frontend/src/App.vue", "new frontend");
    put(tmp.path(), "crates/devcenterctl/src/main.rs", "new cli");
    let new_sha = commit(tmp.path());
    let selected = plan(
        tmp.path(),
        &json!([baseline.clone()]),
        "deployment-cli",
        false,
    )
    .unwrap();
    assert_eq!(selected["selected"], json!(["deployment-cli"]));
    assert_eq!(selected["reused"]["server"]["source_commit"], old_sha);
    let mut newer = baseline.clone();
    newer["version"] = json!("1.1.0");
    newer["source_commit"] = json!(new_sha);
    let mut metadata = json!({});
    for (output, key) in [
        ("server", "devcenter"),
        ("deployment-cli", "devcenterctl"),
        ("connectors", "devcenter_connectors"),
        ("chart", "chart"),
    ] {
        metadata[output] = json!({"version": if output == "deployment-cli" { "1.1.0" } else { "1.0.0" }, "source_commit": if output == "deployment-cli" { &new_sha } else { &old_sha }, "digest":baseline["artifacts"][key]});
    }
    newer["provenance"] = metadata;
    assert_eq!(
        plan(tmp.path(), &json!([newer, baseline]), "auto", false).unwrap()["selected"],
        json!(["server"])
    );
}
#[test]
fn reused_provenance_is_immutable() {
    let (tmp, sha, baseline) = fixture();
    let result = plan(tmp.path(), &json!([baseline.clone()]), "auto", false).unwrap();
    assert_eq!(
        result["reused"]["chart"],
        json!({"version":"1.0.0", "source_commit":sha, "digest":baseline["artifacts"]["chart"]})
    );
}

#[test]
fn publication_requires_every_selected_receipt() {
    use devcenterctl::publication::{Plan, Provenance, complete};
    use std::collections::BTreeMap;
    let (tmp, _, baseline) = fixture();
    put(tmp.path(), "frontend/src/App.vue", "new frontend");
    commit(tmp.path());
    let value = plan(tmp.path(), &json!([baseline.clone()]), "auto", false).unwrap();
    assert!(
        complete(
            serde_json::from_value(value.clone()).unwrap(),
            BTreeMap::new()
        )
        .unwrap_err()
        .to_string()
        .contains("incomplete")
    );
    let receipt = Provenance {
        version: "2.0.0".into(),
        source_commit: value["source_commit"].as_str().unwrap().into(),
        digest: format!("sha256:{}", "e".repeat(64)),
    };
    let manifest = complete(
        serde_json::from_value::<Plan>(value).unwrap(),
        BTreeMap::from([("server".into(), receipt)]),
    )
    .unwrap();
    assert_eq!(manifest.artifacts["chart"], baseline["artifacts"]["chart"]);
    assert_eq!(manifest.provenance["chart"].version, "1.0.0");
    assert_eq!(manifest.provenance["server"].version, "2.0.0");
}

#[test]
fn completed_release_retry_cannot_republish_or_change_source() {
    let (tmp, _, mut baseline) = fixture();
    baseline["version"] = json!("2.0.0");
    assert_eq!(
        plan(tmp.path(), &json!([baseline.clone()]), "server", false).unwrap()["selected"],
        json!([])
    );
    put(tmp.path(), "frontend/src/App.vue", "new frontend");
    commit(tmp.path());
    assert!(
        plan(tmp.path(), &json!([baseline]), "auto", false)
            .unwrap_err()
            .contains("immutable release identifier")
    );
}

#[test]
fn chart_and_connectors_version_bookkeeping_is_noop() {
    let (tmp, _, _) = fixture();
    put(
        tmp.path(),
        "deploy/charts/devcenter/Chart.yaml",
        "name: devcenter\nversion: 1.0.0\nappVersion: 1.0.0\n",
    );
    put(
        tmp.path(),
        "crates/devcenter-connectors/Cargo.toml",
        "[package]\nname = \"devcenter-connectors\"\nversion = \"1.0.0\"\n",
    );
    let sha = commit(tmp.path());
    let mut baseline = fixture().2;
    baseline["source_commit"] = json!(sha);
    put(
        tmp.path(),
        "deploy/charts/devcenter/Chart.yaml",
        "name: devcenter\nversion: 2.0.0\nappVersion: 2.0.0\n",
    );
    put(
        tmp.path(),
        "crates/devcenter-connectors/Cargo.toml",
        "[package]\nname = \"devcenter-connectors\"\nversion = \"2.0.0\"\n",
    );
    commit(tmp.path());
    assert_eq!(
        plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
        json!([])
    );
}

#[test]
fn ess_recipe_changes_follow_output_reachability() {
    for (node, expected) in [
        ("server-root", json!(["server"])),
        ("chart-package", json!(["chart"])),
        (
            "rust-base",
            json!(["connectors", "deployment-cli", "server"]),
        ),
    ] {
        let (tmp, _, baseline) = fixture();
        let mut ir: Value =
            serde_json::from_str(include_str!("../../../generated/ess/build.json")).unwrap();
        ir["nodes"][node]["fixture_change"] = json!(true);
        put(tmp.path(), "generated/ess/build.json", &ir.to_string());
        commit(tmp.path());
        assert_eq!(
            plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
            expected
        );
    }
}

#[test]
fn generated_diagram_changes_do_not_publish() {
    let (tmp, _, baseline) = fixture();
    put(tmp.path(), "generated/ess/build.mmd", "new diagram");
    commit(tmp.path());
    assert_eq!(
        plan(tmp.path(), &json!([baseline]), "auto", false).unwrap()["selected"],
        json!([])
    );
}

#[cfg(unix)]
fn executable(root: &Path, name: &str, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    put(root, name, body);
    fs::set_permissions(root.join(name), fs::Permissions::from_mode(0o755)).unwrap();
}

#[test]
#[cfg(unix)]
fn failed_partial_publications_and_network_errors_cannot_overwrite_tags() {
    let tmp = tempfile::tempdir().unwrap();
    for (body, expected) in [
        ("#!/bin/sh\nexit 0\n", false),
        (
            "#!/bin/sh\necho 'ERROR: ghcr.io/example/runtime:2.0.0: not found' >&2\nexit 1\n",
            true,
        ),
        (
            "#!/bin/sh\necho 'network host not found' >&2\nexit 1\n",
            false,
        ),
        ("#!/bin/sh\necho '401 Unauthorized' >&2\nexit 1\n", false),
    ] {
        executable(tmp.path(), "docker", body);
        let output = Command::new("bash")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../ci/publication-tag-absent.sh"
            ))
            .arg("ghcr.io/example/runtime:2.0.0")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    tmp.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            expected,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
#[cfg(unix)]
fn baseline_transport_fails_closed_and_does_not_use_expiring_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    for (body, expected) in [
        ("#!/bin/sh\nexit 1\n", false),
        ("#!/bin/sh\necho '[]'\n", true),
        (
            "#!/bin/sh\necho '[{\"draft\":false,\"prerelease\":false,\"tag_name\":\"2.0.0\",\"published_at\":\"now\",\"assets\":[]}]'\n",
            false,
        ),
        (
            "#!/bin/sh\nif [ \"$1\" = api ]; then echo '[{\"draft\":false,\"prerelease\":false,\"tag_name\":\"2.0.0\",\"published_at\":\"now\",\"assets\":[{\"name\":\"release-manifest.json\"}]}]'; else exit 1; fi\n",
            false,
        ),
    ] {
        executable(tmp.path(), "gh", body);
        let output = Command::new("bash")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../ci/publication-history.sh"
            ))
            .arg(tmp.path().join("history.json"))
            .env("GITHUB_REPOSITORY", "example/product")
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    tmp.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            expected,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn workflow_preserves_selection_and_candidate_validation_boundaries() {
    let workflow: Value =
        serde_yaml::from_str(include_str!("../../../.github/workflows/release.yml")).unwrap();
    assert_eq!(workflow["jobs"]["chart"]["needs"], "plan");
    for job in ["images", "chart"] {
        assert!(
            workflow["jobs"][job]["if"]
                .as_str()
                .unwrap()
                .contains("needs.plan.outputs.")
        );
    }
    assert!(
        workflow["jobs"]["images"]["strategy"]["matrix"]
            .as_str()
            .unwrap()
            .contains("image_matrix")
    );
    let final_steps = workflow["jobs"]["github-release"]["steps"]
        .as_array()
        .unwrap();
    let publish = final_steps.last().unwrap()["run"].as_str().unwrap();
    assert!(
        publish.find("publication-candidate.sh").unwrap()
            < publish.find("gh release create").unwrap()
    );
    assert!(!publish.contains("--clobber"));
    let promote: Value = serde_yaml::from_str(include_str!(
        "../../../.github/workflows/promote-connectors.yml"
    ))
    .unwrap();
    assert_eq!(
        promote["jobs"]["publish"]["uses"],
        "./.github/workflows/release.yml"
    );
    assert_eq!(promote["jobs"]["publish"]["with"]["unit"], "connectors");
}

#[test]
fn invalid_publication_identifiers_refuse_before_builds() {
    let (tmp, _, baseline) = fixture();
    let history = [serde_json::from_value(baseline).unwrap()];
    assert!(
        devcenterctl::publication::plan(tmp.path(), &history, "--invalid", "auto", false)
            .unwrap_err()
            .to_string()
            .contains("invalid release identifier")
    );
}

#[test]
fn chart_publication_requires_a_valid_helm_version_before_builds() {
    let (tmp, _, baseline) = fixture();
    put(
        tmp.path(),
        "deploy/charts/devcenter/templates/a.yaml",
        "chart update",
    );
    commit(tmp.path());
    let history = [serde_json::from_value(baseline).unwrap()];
    assert!(
        devcenterctl::publication::plan(tmp.path(), &history, "next", "auto", false)
            .unwrap_err()
            .to_string()
            .contains("SemVer")
    );
}

#[test]
#[cfg(unix)]
fn composed_candidate_renders_the_reused_chart_and_checks_its_digest() {
    let (tmp, _, baseline) = fixture();
    let value = plan(tmp.path(), &json!([baseline]), "auto", false).unwrap();
    let manifest = devcenterctl::publication::complete(
        serde_json::from_value(value).unwrap(),
        std::collections::BTreeMap::new(),
    )
    .unwrap();
    put(
        tmp.path(),
        "manifest.json",
        &serde_json::to_string(&manifest).unwrap(),
    );
    let helm = Command::new("bash")
        .args(["-c", "command -v helm"])
        .output()
        .unwrap();
    assert!(
        helm.status.success(),
        "Helm is required by the release candidate gate"
    );
    let helm = String::from_utf8(helm.stdout).unwrap().trim().to_owned();
    let packaged = Command::new(&helm)
        .args([
            "package",
            concat!(env!("CARGO_MANIFEST_DIR"), "/../../deploy/charts/devcenter"),
            "--version",
            "1.0.0",
            "--destination",
        ])
        .arg(tmp.path())
        .output()
        .unwrap();
    assert!(
        packaged.status.success(),
        "{}",
        String::from_utf8_lossy(&packaged.stderr)
    );
    executable(
        tmp.path(),
        "bin/helm",
        "#!/bin/sh\nif [ \"$1\" = pull ]; then\n  test \"$4\" = 1.0.0 || exit 1\n  cp \"$MOCK_CHART\" \"$6/devcenter-1.0.0.tgz\"\n  echo \"Digest: $MOCK_CHART_DIGEST\"\nelse\n  exec \"$ACTUAL_HELM\" \"$@\"\nfi\n",
    );
    let run = |digest: &str| {
        Command::new("bash")
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../ci/publication-candidate.sh"
            ))
            .arg(tmp.path().join("manifest.json"))
            .arg(env!("CARGO_BIN_EXE_devcenterctl"))
            .env("GITHUB_REPOSITORY_OWNER", "example")
            .env("MOCK_CHART", tmp.path().join("devcenter-1.0.0.tgz"))
            .env("MOCK_CHART_DIGEST", digest)
            .env("ACTUAL_HELM", &helm)
            .env(
                "PATH",
                format!(
                    "{}/bin:{}",
                    tmp.path().display(),
                    std::env::var("PATH").unwrap()
                ),
            )
            .output()
            .unwrap()
    };
    let success = run(&manifest.artifacts["chart"]);
    assert!(
        success.status.success(),
        "{}",
        String::from_utf8_lossy(&success.stderr)
    );
    assert!(String::from_utf8_lossy(&success.stdout).contains("deployment validation:"));
    assert!(!run(&format!("sha256:{}", "e".repeat(64))).status.success());
}

#[test]
fn independent_chart_and_connectors_versions_pass_release_consistency() {
    let tmp = tempfile::tempdir().unwrap();
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."));
    for path in [
        "Cargo.toml",
        "frontend/package.json",
        "openapi.json",
        "crates/devcenter-connectors/Cargo.toml",
        "Dockerfile",
        "rust-toolchain.toml",
    ] {
        let mut body = fs::read_to_string(root.join(path)).unwrap();
        if path == "crates/devcenter-connectors/Cargo.toml" {
            let line = body
                .lines()
                .find(|line| line.starts_with("version = "))
                .unwrap()
                .to_owned();
            body = body.replacen(&line, "version = \"9.8.7\"", 1);
        }
        put(tmp.path(), path, &body);
    }
    put(
        tmp.path(),
        "deploy/charts/devcenter/Chart.yaml",
        "version: 4.5.6\nappVersion: \"1.2.3\"\n",
    );
    let output = Command::new("bash")
        .arg(root.join("ci/check-version-consistency.sh"))
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
