---
format: aep.planning-md/1
id: story:reconcile-independent-stack-units
kind: story
status: draft
title: Reconcile independently selected stack units
summary: Lower an exact private stack lock and environment document into affected-only deployments.
relations:
- decomposes: epic:independent-component-delivery
- depends_on: story:ess-release-model
revision: 1
---
## Context

ESS and Devcenter now establish repository-owned semantic and build inputs, but operational delivery
still couples component publication and consumes a conventional aggregate chart. The next slice
belongs at the private composition boundary: exact released units, environment intent, and a
reconciler that computes the changed subset.

## Acceptance

Changing one component's immutable release selection in the private stack lock causes the reconciler to validate the composed ESS obligations and deploy only that component's affected Helm release while every unrelated release remains byte-identical and untouched.

## Scope

- Private stack lock with exact versions and image/chart digests
- Environment document containing only deployment-owned configuration references
- ESS validation of cross-component obligations before lowering
- Deterministic affected-unit calculation and per-release Helm application
- Deployment evidence showing unchanged components were neither rebuilt nor rolled out
