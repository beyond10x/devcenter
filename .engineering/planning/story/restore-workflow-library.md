---
format: aep.planning-md/1
id: story:restore-workflow-library
kind: story
status: active
title: Restore the reusable Workflow library
summary: Make the service-owned Workflow bundle readable without manual database seeding or a user-side installer.
relations:
- derived_from: initiative:engineer-journey
scope:
- confidence: cited
  path: .github/workflows/promote-workflow.yml
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src/features/workflows
revision: 5
---
## Context

The deployed Workflow process is healthy, but the authenticated Devcenter library request is refused and the durable store contains no reusable definitions. The current Devcenter-side “Install starter library” shortcut places service bootstrap ownership in the UI and cannot make a fresh deployment ready by construction.

## Acceptance

After a fresh deployment, an authenticated engineer opens Workflow and can list and inspect the immutable Code review, Security review, and Reverse AEP + ESS graphs from a Workflow-owned, idempotently reconciled resource bundle, with no user click, direct database seed, or Devcenter-owned Workflow write model.

## Scope

The confirmed diagnosis changed no Devcenter product source. The live refusal was reproduced against Workflow 0.3.5 and traced to Service SDK projection validation of an absent optional `active_revision_id`. The repair lives in Service SDK's realization builder and engine; Workflow must consume that release and regenerate its realization plan before Devcenter updates its immutable Workflow reference.

The starter bundle is a distinct remaining scope. Workflow's manifest probe was reverted because Service SDK 0.5.7 rejects `resources` and has no service-owned reconciliation lifecycle. `dependency-blocker:service-sdk-resource-reconciliation` records that contract rather than hiding a database seed in Devcenter.
