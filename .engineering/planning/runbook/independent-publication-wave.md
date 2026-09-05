---
format: aep.planning-md/1
id: runbook:independent-publication-wave
kind: runbook
status: draft
title: Complete independent artifact publication
relations:
- decides: story:selective-artifact-publication
- decides: story:independent-workspace-publication
revision: 1
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

Plan review complete. Implementation pending. Run scoped unit checks, independent adversarial verification, then the repository gate once on each integrated result. Record each command exit independently. No gate evidence is asserted before execution.
