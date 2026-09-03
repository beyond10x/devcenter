---
format: aep.planning-md/1
id: story:workflow-library-read-surface
kind: story
status: implemented
title: Read Workflow library through the official client
summary: Browse standalone Workflow definitions while keeping Workspace project workflows unchanged.
relations:
- derived_from: initiative:engineer-journey
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: ci
- confidence: cited
  path: crates/devcenter-app
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: crates/devcenter-core
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src
- confidence: cited
  path: openapi.json
revision: 7
---
## Outcome

An engineer can browse and inspect standalone Workflow definitions in DevCenter while existing Workspace-backed project workflows continue unchanged.

## Context

Workflow and Workspace are separate product boundaries. DevCenter adds the standalone Workflow library as a compact URL-backed read surface and retains Workspace for project workflow execution.

## Acceptance

- DevCenter 0.8.12 pins Workflow 0.2.0, Service SDK 0.4.2, AgentIDE 0.3.1's generated service and Vue renderer, Todo 0.2.7, Workspace 0.2.13, and the exact Connectors 0.5.6 revision shared by the generated services and hosted runtime.
- The Agent Platform 0.6.7 coding-turn boundary remains on the exact AgentIDE contract it declares; no conversion shim or compatibility adapter is added.
- The BFF exposes only `GET /api/workflows` and `GET /api/workflows/{id}` through the official Workflow client and exchanges exactly `workflows.read` for audience `urn:b10x:workflow`.
- The Vue app provides a compact URL-backed Workflow library with a bold selector, revision and node summary, and no restored fat sidebar.
- Workflow unavailable and refusal states are explicit; tenant and actor remain session-derived.
- The chart can enable Workflow and wires its origin, audience, probes, and Identity configuration without embedding deployment-specific values.
- Contract, browser, chart-disabled, chart-enabled, dependency-graph, and devserver visual checks pass.

## Out of Scope

Workflow authoring in DevCenter, AEP composition, Workflow execution, changes to Workspace project workflows, Platform, Cloud, Atlas, Website, and deployment-specific values.

## Open Questions

None. The first DevCenter slice is read-only; authoring remains available through the official Workflow client.

## Scope

- cited: BFF config/routes, frontend router/API/components/styles/tests and visual snapshot, root and isolated Connector manifests, Helm templates/tests, OpenAPI, README, changelog, and version metadata.
- inferred: the existing broad `story:workflow-aep-composition` remains open after this narrower slice lands.
