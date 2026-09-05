---
format: aep.planning-md/1
id: review-result:selective-publication-pass-1
kind: review-result
status: active
title: Selective publication adversarial pass 1
relations:
- reviews: story:selective-artifact-publication
revision: 1
---
unit: selective-artifact-publication — frozen working tree on e49a5984d7403280369817b8397d8c7b8a2f955a
verdict: nothing found
cases: executed 27→30, red 0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: 1 persistent report path; temporary fixture directories and normal Cargo/sccache outputs described below
needs-coordinator: none

1. Diff ownership

`git --no-pager diff --stat` (the pre-existing, implementor-owned working diff remains present):

```text
 .github/workflows/gate.yml               |  34 +--
 .github/workflows/promote-connectors.yml | 132 +---------
 .github/workflows/release.yml            | 412 +++++++++++++++++--------------
 ci/check-release-unit-impact.sh          |   3 +
 ci/check-version-consistency.sh          |  11 +-
 ci/release-unit-impact.sh                |  73 +-----
 crates/devcenterctl/src/lib.rs           |   1 +
 crates/devcenterctl/src/main.rs          |  81 ++++++
 8 files changed, 346 insertions(+), 401 deletions(-)
```

This tree was supplied with an uncommitted implementation. The non-test paths above are unchanged by this pass and match the supplied implementor report. My entire source-tree addition is the previously absent test file, measured with `git diff --no-index --stat -- /dev/null crates/devcenterctl/tests/publication_adversary.rs`:

```text
 .../devcenterctl/tests/publication_adversary.rs    | 207 +++++++++++++++++++++
 1 file changed, 207 insertions(+)
```

No implementation edits, old assertion edits, planning commands, commits, remote calls, or worktree lifecycle operations were performed. `git diff --check` exited 0.

2. Cases added and executed individually before the suite

All three cases are in `crates/devcenterctl/tests/publication_adversary.rs`. The first exercises simultaneous source YAML, generated IR, and Dockerfile projection changes for server, CLI, and chart recipes. The second constructs a non-ancestor artifact provenance inside a disposable fixture repository and requires refusal. The third executes the real history adapter with a local `gh` fixture returning multiple release pages, including a failed draft and a requested older identifier, then drives the planner with the resulting manifests.

Each command used the following prefix from the assigned worktree:

```console
RUSTC_WRAPPER=/usr/bin/sccache TMPDIR=/home/timo/.cache/independent-publication-wave.e1Bll4/publication cargo test -p devcenterctl --test publication_adversary --locked <case> -- --exact
```

Case `source_and_generated_recipe_changes_keep_unrelated_outputs_reused`, first run, exit 0:

```text
   Compiling devcenterctl v0.8.17 (/home/timo/.local/state/worktree/trees/b10x/devcenter/selective-artifact-publication/crates/devcenterctl)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.42s
     Running tests/publication_adversary.rs (target/debug/deps/publication_adversary-348214603c5912a8)

running 1 test
test source_and_generated_recipe_changes_keep_unrelated_outputs_reused ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.20s
```

Case `reused_provenance_from_unrelated_branch_is_refused`, first run, exit 0:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running tests/publication_adversary.rs (target/debug/deps/publication_adversary-348214603c5912a8)

running 1 test
test reused_provenance_from_unrelated_branch_is_refused ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.03s
```

Case `paginated_transport_retains_latest_and_requested_baselines_only`, first run, exit 0:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.10s
     Running tests/publication_adversary.rs (target/debug/deps/publication_adversary-348214603c5912a8)

running 1 test
test paginated_transport_retains_latest_and_requested_baselines_only ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out; finished in 0.05s
```

No red output exists. Baseline executed count 27 comes from the implementor's supplied report, not a premature suite run.

3. Package suite after the cases

Command: `RUSTC_WRAPPER=/usr/bin/sccache TMPDIR=/home/timo/.cache/independent-publication-wave.e1Bll4/publication cargo test -p devcenterctl --locked`. Exit 0.

```text
   Compiling devcenterctl v0.8.17 (/home/timo/.local/state/worktree/trees/b10x/devcenter/selective-artifact-publication/crates/devcenterctl)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.29s
     Running unittests src/lib.rs (target/debug/deps/devcenterctl-f2e967b7f20a70b7)

running 6 tests
test deployment::tests::disabled_required_components_are_refused ... ok
test deployment::tests::docker_hub_shorthand_matches_the_canonical_lock_reference ... ok
test leak::tests::excludes_the_runtime_denylist_from_its_own_scan ... ok
test deployment::tests::mutable_or_unlocked_images_are_refused ... ok
test leak::tests::reports_location_without_echoing_marker ... ok
test deployment::tests::every_rendered_image_and_required_component_must_be_locked ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/devcenterctl-a2b1e77abfcceae1)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/publication.rs (target/debug/deps/publication-dbe910ef38a0c350)

running 21 tests
test invalid_publication_identifiers_refuse_before_builds ... ok
test workflow_preserves_selection_and_candidate_validation_boundaries ... ok
test absent_history_requires_explicit_bootstrap ... ok
test independent_chart_and_connectors_versions_pass_release_consistency ... ok
test malformed_or_incomplete_history_refuses ... ok
test generated_diagram_changes_do_not_publish ... ok
test documentation_is_noop ... ok
test completed_release_retry_cannot_republish_or_change_source ... ok
test failed_partial_publications_and_network_errors_cannot_overwrite_tags ... ok
test missing_source_history_refuses ... ok
test chart_publication_requires_a_valid_helm_version_before_builds ... ok
test baseline_transport_fails_closed_and_does_not_use_expiring_artifacts ... ok
test reused_provenance_is_immutable ... ok
test chart_and_connectors_version_bookkeeping_is_noop ... ok
test publication_requires_every_selected_receipt ... ok
test real_dependency_change_selects_dependents ... ok
test own_release_version_does_not_rebuild ... ok
test explicit_unit_does_not_mask_staggered_changes ... ok
test ess_recipe_changes_follow_output_reachability ... ok
test independent_surface_selection ... ok
test composed_candidate_renders_the_reused_chart_and_checks_its_digest ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.19s

     Running tests/publication_adversary.rs (target/debug/deps/publication_adversary-348214603c5912a8)

running 3 tests
test reused_provenance_from_unrelated_branch_is_refused ... ok
test paginated_transport_retains_latest_and_requested_baselines_only ... ok
test source_and_generated_recipe_changes_keep_unrelated_outputs_reused ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s

   Doc-tests devcenterctl

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Command: `RUSTC_WRAPPER=/usr/bin/sccache TMPDIR=/home/timo/.cache/independent-publication-wave.e1Bll4/publication cargo clippy -p devcenterctl --all-targets --locked -- -D warnings`. Exit 0.

```text
    Checking devcenterctl v0.8.17 (/home/timo/.local/state/worktree/trees/b10x/devcenter/selective-artifact-publication/crates/devcenterctl)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
```

4. Judgement findings

Nothing found in this pass.

5. Attacks and limits

Simultaneous ESS source/IR/Dockerfile recipe changes select only the intended output in the three executable scenarios; all three other provenances remain reused.

Non-ancestor provenance is refused before publication even when it belongs to an unselected output.

Paginated history excludes drafts, preserves newest-first ordering, retrieves the requested historical publication, and drives a completed same-source/id retry to a no-op.

Existing targeted cases also cover missing receipts, unknown baselines, independent chart scheduling, failed registry lookups, immutable tags, staggered baselines, and real Helm rendering with a reused chart. The real registry, GitHub Actions scheduler, and source-integration gate were not executed. Source/IR drift remains covered by the existing `ci/check-ess-model.sh` integration gate, read during this pass.

Partial registry pushes require a fresh identifier, as declared by the implementation. The stated acceptance requires immutable outputs and failed-baseline refusal; it does not require resuming a partially pushed identifier. I found no acceptance contradiction in that conservative retry behavior.

6. Outside-worktree writes

Persistent report: `/home/timo/.cache/independent-publication-wave.e1Bll4/publication/adversary-pass-1.md`.

Disposable fixture repositories and shell transport scratch were created beneath the assigned `/home/timo/.cache/independent-publication-wave.e1Bll4/publication` via inherited `TMPDIR`, then removed by tempfile lifetime/shell traps. No new fixture directory was retained. Test compiler output remains in the supplied worktree's `target`; Cargo and `/usr/bin/sccache` used their normal configured caches, without an alternate target directory.

```findings
[]
```
