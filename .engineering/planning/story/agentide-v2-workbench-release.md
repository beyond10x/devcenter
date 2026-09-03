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
  path: frontend/review-e2e/hosted-workbench.spec.ts
- confidence: inferred
  path: frontend/src/api/client.ts
- confidence: inferred
  path: frontend/src/features/workbench/HostedTerminal.vue
- confidence: inferred
  path: frontend/src/features/workbench/HostedWorkspaceView.vue
- confidence: inferred
  path: openapi.json
revision: 11
---
# Release the hosted workbench on AgentIDE v2

## Context

DevCenter now consumes Agent Platform 0.6.7, AgentIDE 0.2.1, Workspace 0.2.12, and Connectors v0.5.6
through fixed release tags and one canonical HTTPS Cargo source graph. Its composed generated-service
binary consumes the AgentIDE 0.2.1 and Todo 0.2.6 tags. Service SDK 0.3.4 remains the generated
service factory and Eventlog remains its persistence seam. Their direct source revisions, including
the compatible Connectors runtime identity, are retained only because the released AgentIDE and Todo
generated manifests select the same Service SDK source identity, Service SDK selects that Connectors
factory identity, and Service SDK selects an untagged Eventlog source. Changing only DevCenter's
spelling creates duplicate, incompatible Rust trait types.

Workspace remains the sole file, diff, process, and PTY authority. DevCenter derives actor identity,
recomputes complete-content digests, seals browser selection drafts, and recreates the Ghostty browser
renderer when its theme changes while reconnecting to the same confined Workspace terminal.

## Acceptance

DevCenter 0.8.11 passes the complete repository and browser gates, Cargo reports one source identity
for each shared AgentIDE, Agent Platform, Workspace, and Connectors contract, and the coordinator can
promote a workbench whose Monaco syntax colors and Ghostty ANSI palette remain visible after theme
changes and replay.

## Scope

Dependency manifests and locks, hosted selection sealing, Ghostty renderer lifecycle, browser color
assertions, release metadata, and coordinator handoff evidence.
