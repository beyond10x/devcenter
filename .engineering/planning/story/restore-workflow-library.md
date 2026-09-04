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
revision: 4
---
## Context

The deployed Workflow process is healthy, but the authenticated Devcenter library request is refused and the durable store contains no reusable definitions. The current Devcenter-side “Install starter library” shortcut places service bootstrap ownership in the UI and cannot make a fresh deployment ready by construction.

## Acceptance

After a fresh deployment, an authenticated engineer opens Workflow and can list and inspect the immutable Code review, Security review, and Reverse AEP + ESS graphs from a Workflow-owned, idempotently reconciled resource bundle, with no user click, direct database seed, or Devcenter-owned Workflow write model.

## Scope

Consume and promote the released Workflow-owned bundle, preserve the exact downstream refusal in safe diagnostics, remove the user-side bootstrap requirement, and verify the authenticated read path.
