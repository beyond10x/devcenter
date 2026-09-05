use devcenterctl::publication::{Manifest, plan};
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
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn write(root: &Path, path: &str, body: &str) {
    fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
    fs::write(root.join(path), body).unwrap();
}
fn commit(root: &Path) -> String {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "commit",
            "-qm",
            "fixture",
        ],
    );
    git(root, &["rev-parse", "HEAD"])
}
fn fixture() -> (tempfile::TempDir, Manifest) {
    let tmp = tempfile::tempdir().unwrap();
    git(tmp.path(), &["init", "-q"]);
    for (path, body) in [
        ("ess/build.yaml", include_str!("../../../ess/build.yaml")),
        (
            "generated/ess/build.json",
            include_str!("../../../generated/ess/build.json"),
        ),
        ("Dockerfile.ess", include_str!("../../../Dockerfile.ess")),
        ("docker-bake.hcl", include_str!("../../../docker-bake.hcl")),
    ] {
        write(tmp.path(), path, body);
    }
    let sha = commit(tmp.path());
    let baseline = serde_json::from_value(
        json!({"schema":1,"version":"1.0.0","source_commit":sha,"artifacts":{
        "devcenter":format!("sha256:{}", "a".repeat(64)),
        "devcenterctl":format!("sha256:{}", "b".repeat(64)),
        "devcenter_connectors":format!("sha256:{}", "c".repeat(64)),
        "chart":format!("sha256:{}", "d".repeat(64))}}),
    )
    .unwrap();
    (tmp, baseline)
}

#[test]
fn source_and_generated_recipe_changes_keep_unrelated_outputs_reused() {
    for (node, expected) in [
        ("server-root", vec!["server"]),
        ("ctl-root", vec!["deployment-cli"]),
        ("chart-package", vec!["chart"]),
    ] {
        let (tmp, baseline) = fixture();
        let mut source: Value =
            serde_yaml::from_str(include_str!("../../../ess/build.yaml")).unwrap();
        let mut ir: Value =
            serde_json::from_str(include_str!("../../../generated/ess/build.json")).unwrap();
        let source_node = source["nodes"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|n| n["id"] == node)
            .unwrap();
        let old = source_node["argv"][2].as_str().unwrap().to_owned();
        let new = format!("{old} && true");
        source_node["argv"][2] = json!(new);
        ir["nodes"][node]["argv"][2] = json!(new);
        write(
            tmp.path(),
            "ess/build.yaml",
            &serde_yaml::to_string(&source).unwrap(),
        );
        write(tmp.path(), "generated/ess/build.json", &ir.to_string());
        let docker = include_str!("../../../Dockerfile.ess");
        let old_json = serde_json::to_string(&old).unwrap();
        let new_json = serde_json::to_string(&new).unwrap();
        assert!(docker.contains(&old_json));
        write(
            tmp.path(),
            "Dockerfile.ess",
            &docker.replace(&old_json, &new_json),
        );
        commit(tmp.path());
        let result = plan(tmp.path(), &[baseline], "2.0.0", "auto", false).unwrap();
        assert_eq!(result.selected.into_iter().collect::<Vec<_>>(), expected);
        assert_eq!(result.reused.len(), 3);
    }
}

#[test]
fn reused_provenance_from_unrelated_branch_is_refused() {
    let (tmp, baseline) = fixture();
    let base = baseline.source_commit.clone();
    write(tmp.path(), "frontend/src/App.vue", "other branch");
    let foreign = commit(tmp.path());
    // This is an isolated fixture repository, never the checkout under attack.
    git(tmp.path(), &["checkout", "--detach", &base]);
    write(tmp.path(), "frontend/src/App.vue", "candidate branch");
    commit(tmp.path());
    let mut manifest = serde_json::to_value(&baseline).unwrap();
    for (name, key) in [
        ("server", "devcenter"),
        ("deployment-cli", "devcenterctl"),
        ("connectors", "devcenter_connectors"),
        ("chart", "chart"),
    ] {
        manifest["provenance"][name] = json!({"version":"1.0.0", "source_commit":if name == "chart" { &foreign } else { &base }, "digest":baseline.artifacts[key]});
    }
    let error = plan(
        tmp.path(),
        &[serde_json::from_value(manifest).unwrap()],
        "2.0.0",
        "server",
        false,
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("Git history unavailable"),
        "{error}"
    );
}

#[test]
#[cfg(unix)]
fn paginated_transport_retains_latest_and_requested_baselines_only() {
    use std::os::unix::fs::PermissionsExt;
    let (tmp, baseline) = fixture();
    write(
        tmp.path(),
        "baseline.json",
        &serde_json::to_string(&baseline).unwrap(),
    );
    write(
        tmp.path(),
        "bin/gh",
        r#"#!/usr/bin/env bash
set -eu
if [ "$1" = api ]; then
  test "$2" = --paginate
  printf '%s\n' '[{"draft":false,"prerelease":false,"tag_name":"publication-2.0.0","published_at":"2026-01-03","assets":[{"name":"release-manifest.json"}]},{"draft":true,"prerelease":false,"tag_name":"publication-failed","published_at":"2026-01-04","assets":[]}]'
  printf '%s\n' '[{"draft":false,"prerelease":false,"tag_name":"1.1.0","published_at":"2026-01-02","assets":[{"name":"release-manifest.json"}]},{"draft":false,"prerelease":false,"tag_name":"1.0.0","published_at":"2026-01-01","assets":[{"name":"release-manifest.json"}]}]'
else
  test "$1" = release
  test "$2" = download
  tag=$3
  shift 3
  while [ "$1" != --dir ]; do shift; done
  jq --arg version "${tag#publication-}" '.version = $version' "$FIXTURE_BASELINE" > "$2/release-manifest.json"
fi
"#,
    );
    fs::set_permissions(tmp.path().join("bin/gh"), fs::Permissions::from_mode(0o755)).unwrap();
    let destination = tmp.path().join("transport-history.json");
    let output = Command::new("bash")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../ci/publication-history.sh"
        ))
        .arg(&destination)
        .arg("1.0.0")
        .env("GITHUB_REPOSITORY", "example/product")
        .env("FIXTURE_BASELINE", tmp.path().join("baseline.json"))
        .env(
            "PATH",
            format!(
                "{}/bin:{}",
                tmp.path().display(),
                std::env::var("PATH").unwrap()
            ),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let history: Vec<Manifest> = serde_json::from_slice(&fs::read(destination).unwrap()).unwrap();
    assert_eq!(
        history
            .iter()
            .map(|m| m.version.as_str())
            .collect::<Vec<_>>(),
        ["2.0.0", "1.0.0"]
    );
    let result = plan(tmp.path(), &history, "1.0.0", "auto", false).unwrap();
    assert!(result.selected.is_empty());
    assert!(result.reused.values().all(|p| p.version == "1.0.0"));
}
