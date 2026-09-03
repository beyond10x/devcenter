---
format: aep.planning-md/1
id: story:agentide-v2-workbench-release
kind: story
status: implemented
title: Release hosted workbench on AgentIDE v2
summary: Pin, validate, release, and deploy the sealed hosted coding protocol.
relations:
- derived_from: story:hosted-coding-workbench
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/devcenter-connectors/Cargo.lock
- confidence: inferred
  path: crates/devcenter-connectors/Cargo.toml
- confidence: inferred
  path: crates/devcenter-http/Cargo.toml
- confidence: inferred
  path: crates/devcenter-http/src/lib.rs
- confidence: inferred
  path: deploy/charts/devcenter/Chart.yaml
- confidence: inferred
  path: frontend/e2e/devcenter.spec.ts
- confidence: inferred
  path: frontend/package.json
- confidence: inferred
  path: frontend/pnpm-lock.yaml
- confidence: inferred
  path: frontend/src/api/client.ts
- confidence: inferred
  path: frontend/src/features/workbench/HostedWorkspaceView.vue
- confidence: inferred
  path: openapi.json
revision: 8
---
# Release the hosted workbench on AgentIDE v2

## Context

AgentIDE 0.2.1, Workspace 0.2.9, and Agent Platform 0.6.6 are aligned as one candidate graph on Harness 0.11.1, Service SDK 0.3.4, and Todo 0.2.6. DevCenter consumes the AgentIDE renderer draft directly and seals authenticated, content-revision-bound selections before submitting each coding turn. Workspace remains the only file, diff, process, and PTY authority; AgentIDE coordination remains on the generated Service SDK and Eventlog seam.

## Acceptance

DevCenter 0.8.8 passes its complete repository and browser gates, publishes immutable runtime artifacts pinned to the released dependency graph, and the private dev deployment verifies a usable editor, diff, and confined terminal without a bind step.

## Scope

Dependency locks, hosted workbench compatibility, release metadata, browser fixtures, and deployment evidence.
