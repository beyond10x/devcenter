---
format: aep.planning-md/1
id: runbook:independent-publication-wave
kind: runbook
status: draft
title: Complete independent artifact publication
relations:
- decides: story:selective-artifact-publication
- decides: story:independent-workspace-publication
revision: 5
---
## Authorization and selection

Interactive run, pre-approved by the operator's request to complete this with aep-planning and adp:wave, following standing instructions to integrate to main and clean managed trees. Skill version 0.7.0. Two source commits, integration merges, opening and closing store commits, publication to main and cleanup are authorized by that instruction.

Wave: story:independent-workspace-publication and story:selective-artifact-publication; both serve O4 in AGENTS.md. Existing ESS release outputs are already modeled in ess/build.yaml. No new domain semantics are introduced.

The computed waves command (proposed stories) returns:

- waves: [story:independent-workspace-publication, story:selective-artifact-publication]
- collisions: []
- unassessed: [story:durable-task-recovery, story:governed-run-record, story:workflow-aep-composition]
- cycles: []

Unassessed entries are pre-existing unrelated backlog; they are excluded from this authorized publication wave. Cited source paths and inferred new paths are recorded on both stories.

## Execution

Integration branch wave/independent-publication at fec766248afb34660db1b2e12a6370c83a504649. Managed checkout id independent-publication-wave.

- Devcenter unit: impl/selective-artifact-publication, managed checkout id selective-artifact-publication. Build directory target within that tree.
- Workspace unit: impl/independent-runtime-publication, managed checkout id independent-workspace-publication in Workspace, base 442add4f591348aa5737a5158045252ca6d90d34. Build directory target within that tree.
- Scratch root: agent cache independent-publication-wave.e1Bll4; per-unit subdirectories publication and workspace. Absolute machine paths remain outside committed public records; worktree ids resolve them.

Coordinator owns all AEP mutations, docs/ess-deployment-model.md, and retirement/redirection of promote-workspace.yml. Implementation agents own source and tests only. Managed worktree CLI owns cleanup.

Use aep-drive:implementor and aep-drive:adversary charters through the available generic subagent transport, which cannot select plugin subagent_type. Four independent aep-plan critic perspectives ran in two scheduling batches because three child slots are available; no critic saw another verdict. All four approved, recorded separately.

## Preflight

84 GiB free at start, 20 GiB floor. Two implementors maximum, with one slot reserved for review. Existing measured Rust output: Substrate 833 MiB, Connectors 1.3 GiB. Existing classifier gate takes 0.025 s. Compiler cache sccache is installed and will be set for this wave. Each build stays in its own target directory.

Primary checkouts contain unrelated changes and are preserved. Devcenter: frontend/package.json, frontend/pnpm-lock.yaml, frontend/src/env.d.ts, frontend/src/router/index.ts and frontend/src/features/phone/. Workspace: AGENTS.md, README.md, .github/, b10x.docs.yaml. Wave branches originate from fetched origin/main through managed trees, per standing user instruction.

Five previous completed worktrees were GC-reviewed and removed using exact ids: git-fetch-sessions, git-workspace-materialization, hosted-browser-workbench, real-agentide-workspace, single-git-materialization. Their heads remain recoverable from published branches/tags.

## Gate and status

Plan review complete. Both implementation units passed independent adversarial review. Devcenter source commit 738b3b6 was merged as 2f4bfd6 and published to the default branch through 0eed3b4. Workspace source and release evidence are published through 2cb2d07, tagged 0.2.18. Devcenter frontend tree is unchanged by the units: frozen install exit 0; frontend check exit 0, 36 Vitest cases; browser gate exit 0, 17 passed and 13 existing project-specific exclusions. ESS 0.9.2 generation, chart lint and chart rollout checks exit 0. Source inspection confirms each browser exclusion targets the other desktop/mobile project.

The coordinator provisioned Workspace's image repository variable through repository administration; existing bot secrets were already present. No credentials or deployment coordinates are recorded here.

Integrated root cargo fmt, clippy and test commands all exited 0: 66 tests, including the actual PostgreSQL contract. Nested Connectors fmt, clippy and test commands all exited 0: 3 tests. Publication impact fixtures, version consistency, ESS generation, chart lint/rollouts, workflow actionlint and leak checks all exited 0. The dedicated PostgreSQL gate container was stopped after the gate.

Remote CLI-only publication run 33935069359 succeeded. Real release metadata was downloaded and compared with 0.8.17: only devcenterctl changed, to sha256:49aa2c5af0a6bcbdf0eb92c905c3039fcd8ee6deef9bbfad0333585514eae9ad. Server, Connectors and chart retain their exact original versions, source and digests. Chart job was skipped. Finalization initially hit protected tag creation rules; the coordinator created that exact source tag as bot and reran only the failed final job, without rebuilding images. Source correction 6304349 scopes the bot token to future final release writes. Its additional regression brings deployment-CLI tests to 31, all green; correction and evidence are on the default branch through 1d16549.

Workspace initial owner release run 33934695619 refused before build because repository-scoped GitHub credentials cannot enumerate organization packages to prove first-package absence. Existing administrator read-only access confirmed the configured target was absent. A scoped exact image/source bootstrap correction was tested and published through f947bfc. Recovery run 33935389058 built and health/readiness-smoked ARM64 successfully, but its first push produced a PUBLIC package; strict post-push privacy verification refused publication. The reason for the platform's initial public visibility is not established. The coordinator cancelled the remaining run, removed the one-use bootstrap variable, deleted only the newly created unused package, and confirmed target absence. No existing deployment, source repository visibility, or released image was changed. The deleted registry object is recoverable through provider package restoration, but must not be restored into public service.

The initial-publication allowance was removed in Workspace 53051b4, published through 5a4981a: owner CI verifies an already private target before allocating image builds. An administrator initialized a distinct target with an empty scratch marker and confirmed private visibility and repository access; no application bytes were included in that setup. Owner release run 33935985625 then succeeded for both native architectures, runtime smoke, private publication, signature verification and durable metadata. Downloaded metadata binds version 0.2.18 to source 2cb2d07c73e6bebe7623aaeb84d143089d392e9f and index sha256:128b78076ba9833f955a32a9e2238ba48f8f5e96658e99fd4fef4f851e479c96. Same-source retry 33936402567 succeeded with images and publish jobs skipped. The duplicate Devcenter promote-workspace.yml publisher is removed after that verification. Both selected stories are implemented; deployment selection remains downstream.

Managed selective-artifact-publication was finished and GC-removed after its source was published. Generated nested Connectors and Workspace test outputs and the local smoke image were cleaned; the integration and owner worktrees will be finished after the completion evidence is published. Unrelated newer Devcenter default-branch work is preserved during integration.

Installed AEP validation reports the four new review records as lacking findings blocks despite their verbatim empty findings blocks. This is an observed parser diagnostic, not missing reviewer returns. Existing unrelated diagnostics remain three proposed stories with no scope and one legacy review without a findings block; store validation exits 0.


## Documentation delivery verification

The clean managed Atlas checkout at remote main a8fb936ddcb35c8971311610e5c63cc86d612fab ran docs reconcile --check with explicit managed source overrides. The initial stale primary Docs System collector rejected v4; retrying with the current managed Docs System checkout progressed to: `refused: documentation surface aep/docs differs from its repository manifest`. This organization catalog mismatch is outside the two publication stories. Website source-lock refresh and delivery gate results are recorded separately as they complete.
