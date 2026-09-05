---
format: aep.planning-md/1
id: runbook:projects-recovery-wave
kind: runbook
status: draft
title: Projects recovery execution and verification
relations:
- implements: story:projects-connection-recovery
revision: 2
---
## Authorized repair wave

Skill version 0.7.0. The operator chose Projects repair and a one-unit budget, then explicitly authorized execution after Plan mode ended. This authorizes the opening coordination commits, one repair commit per affected repository, integration merges and closing evidence commits, publication through bot-authored pull requests, required releases and deployment, and exact task cleanup. Source and deployment remain separate verifiable stages.

## Selection and observations

One logical unit: story:projects-connection-recovery, with Devcenter and Workspace changes coordinated sequentially. It serves O1 and O4. The confirmed symptom is an authenticated GET /api/repositories returning 502. Pod health is not user-flow validation.

The draft wave calculation returned these exact lists before the incident took priority:

```json
{"waves":[],"collisions":[],"unassessed":["story:nucleo-global-search","story:phone-widget"]}
```

The proposed-story calculation returned:

```json
{"waves":[],"collisions":[],"unassessed":["story:durable-task-recovery","story:governed-run-record","story:workflow-aep-composition"]}
```

Read-only scopers assessed all five. Nucleo search is implementable but deferred for the incident. Phone integration and pinning are already present; its draft evidence is stale. Durable recovery and governed run records require unpublished upstream contracts. Workflow composition overlaps existing implemented reads and needs a narrower remaining contract. Those feature scopes were not persisted during Plan mode and those stories are not dispatched by this incident wave.

## Preflight and execution

The clean primary Devcenter checkout is stale; managed trees start from remote main bb7311b0a7bc03c61714e89f5379014d88c83210 and Workspace 069f6137bbb3dea9e4badd8dde386326a29af537. Existing older worktrees belong to other sessions and are untouched. Disk at execution preflight: 112 GiB available, minimum remaining floor 40 GiB. RUSTC_WRAPPER uses installed sccache; CARGO_BUILD_JOBS=2; distinct target directories per tree. One measured build will be recorded with its result before increasing concurrency. Model budget is one implementor then one reviewer.

Dispatch charters: aep-drive:implementor and aep-drive:adversary, version 0.7.0. Capability deviation: this harness has no typed plugin-agent dispatch field; generic agents explicitly read those exact charter files and receive a persisted brief.

The source trees use wave/projects-recovery then impl/projects-connection-recovery. Worktree IDs are projects-recovery-devcenter-20260905 and projects-recovery-workspace-20260905, recorded by the manager; build paths are each tree's target; scratch paths are each tree's .scratch/projects-recovery. Private runtime evidence and browser state stay under the task-owned cache root, never in a public commit. Stage: diagnosis and regression reproduction; no implementation yet.

## Completion evidence required

Regression failure before implementation, affected package checks, independent adversarial findings, each required integration gate step with its own exit status, immutable runtime image and rollout evidence, and authenticated hosted Projects/repository navigation verification. Until the hosted flow is verified, this story stays active. Cleanup preserves all evidence before finishing and garbage collecting only exact reviewed task worktree IDs.

## Runtime publication scope

Publish Devcenter server 0.8.21 and Workspace runtime 0.2.20 after review and integration gates. The Workspace public client contract does not change, so Devcenter retains its released 0.2.19 client pins. Devcenter release publication selects server explicitly and composes the existing chart, Connectors and deployment CLI receipts. No Connectors provider rebuild is required by this repair.

Frontend red tests reproduced missing recovery guidance and the engineering-plan-specific error text. Green runs passed 46 unit cases and 27 browser cases with 15 pre-existing platform-specific skips. Main Rust checks include the PostgreSQL store contract against a task-owned PostgreSQL 17 fixture. The repository-pinned ESS 0.9.2 build comparison passed.

Read-only organization documentation reconciliation used clean Atlas remote main 7b67e8e2437ec9956135930435875a8a76139c3f with explicit managed Workspace and Devcenter roots. It refuses the pre-existing AgentIDE v4 manifest through the pinned older Docs System collector. This repair changes no public documentation source or declaration; no organization-wide documentation convergence is claimed.
