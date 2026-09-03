---
format: aep.planning-md/1
id: story:hosted-coding-workbench
kind: story
status: implemented
title: Deliver the hosted AgentIDE coding workbench
summary: Compose Workspace files, diffs and terminals with Service SDK coordination in the native DevCenter project surface.
relations:
- derived_from: initiative:engineer-journey
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: frontend/review
- confidence: cited
  path: frontend/review-e2e
- confidence: cited
  path: frontend/src/features/workbench
- confidence: cited
  path: openapi.json
revision: 6
---
# Story: Deliver the hosted AgentIDE coding workbench

## Outcome

An authenticated engineer opens a project workspace in DevCenter and receives a native editor, canonical Workspace diff, AgentIDE context and grants, and a confined Substrate terminal without binding a second storage or eventing system.

## Acceptance

- Workspace remains authoritative for project materialization, files, digests, diffs, processes, and PTY sessions.
- AgentIDE coordination is created deterministically for each Workspace session and persists grants, pins, approval checkpoints, and lifecycle events through the generated Service SDK connector and Eventlog.
- The browser renders Monaco and lazily loaded ghostty-web from self-hosted assets, keeps the terminal dock visible, and reconnects stale terminal sessions within the bounded replay contract.
- Actor, tenant, project, session, and grant boundaries are server-derived and fail closed.
- The feature remains deployment-gated until Rust, frontend, browser, chart, leak, and dev-cluster smoke gates pass.

## Scope

DevCenter BFF composition, native workbench UI, hosted terminal transport, review fixtures, release wiring, and deployment gating.
