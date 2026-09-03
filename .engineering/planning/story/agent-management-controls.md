---
format: aep.planning-md/1
id: story:agent-management-controls
kind: story
status: active
title: Remove agents deliberately
summary: Add governed agent retirement once Agent Platform defines the operation.
relations:
- derived_from: epic:authenticated-control-plane
- informed_by: story:agent-platform-journey
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-http/src/lib.rs
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/features/agents
- confidence: cited
  path: frontend/src/stores/workspace.ts
- confidence: cited
  path: frontend/src/styles/main.css
- confidence: cited
  path: frontend/tests/workspace.test.ts
- confidence: cited
  path: openapi.json
revision: 7
---
# Story: Remove agents deliberately

## Outcome

An authenticated engineer can deliberately retire an agent without losing immutable task and revision history or hiding only local browser state.

## Acceptance

- The selected agent exposes a deliberate remove action with confirmation.
- A successful retirement selects a remaining agent or shows the empty state; a refusal preserves the current selection.
- Removal is an authenticated Agent Platform operation exposed through the allowlisted DevCenter BFF.
- The operation defines behavior for retained tasks, active attempts, triggers, and immutable revisions.
- Store, BFF, client, and browser regression coverage proves confirmation, success, and refusal behavior.

## Scope

- Selected-agent removal and confirmation UI.
- Workspace removal state and route reconciliation.
- DevCenter API client, allowlisted BFF route, generated OpenAPI contract, and Agent Platform dependency once the upstream operation exists.
- Focused unit, BFF, and browser regression coverage.
