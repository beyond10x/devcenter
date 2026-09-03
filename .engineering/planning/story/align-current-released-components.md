---
format: aep.planning-md/1
id: story:align-current-released-components
kind: story
status: implemented
title: Align DevCenter with current released components
summary: Pin the mutually compatible current releases used by DevCenter without crossing unsupported service boundaries.
relations:
- derived_from: epic:independent-component-delivery
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/devcenter-connectors
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
  path: frontend/src/api/schema.gen.ts
- confidence: inferred
  path: frontend/src/features/workbench/HostedWorkspaceView.vue
- confidence: inferred
  path: openapi.json
revision: 9
---
## Outcome

DevCenter builds and ships against the current mutually compatible releases of Agent Platform,
AgentIDE, Workspace, the Connectors client, Service SDK, and Todo while preserving the composed
Connectors runtime and Eventlog at the exact revisions required by that released dependency graph.

## Acceptance

- Agent Platform 0.6.6, AgentIDE 0.2.1, Workspace 0.2.10, Connectors client 0.5.4, Service SDK 0.3.4, and Todo 0.2.6 are pinned by immutable release identity.
- Identity remains at current release 0.5.6.
- The composed Connectors runtime remains at revision `03a9f3de1ea2001f057face1ce469a66088bc31b` because Service SDK 0.3.4 publishes its factory traits against that exact graph; Eventlog likewise remains at the Service SDK-selected revision.
- DevCenter adapts to the sealed AgentIDE context contracts without weakening tenant, actor, audience, or scope handling.
- Frontend, browser, Rust, connector, chart, version-consistency, and leak gates pass from the exact dependency graph.
- Workflow is not represented as integrated until its owner releases a supported client, audience, operations, and scopes.

## Compatibility boundary

Connectors 0.5.4 can be used by DevCenter's independent BFF client. It cannot replace the composed
runtime revision until Service SDK publishes generated service factories against that release;
mixing them creates two incompatible `ConnectorServiceFactory` trait identities.
