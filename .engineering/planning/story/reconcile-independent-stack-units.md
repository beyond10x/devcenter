---
format: aep.planning-md/1
id: story:reconcile-independent-stack-units
kind: story
status: active
title: Reconcile independently selected stack units
summary: Lower an exact private stack lock and environment document into affected-only deployments.
relations:
- decomposes: epic:independent-component-delivery
- depends_on: story:ess-release-model
scope:
- confidence: cited
  path: .gitlab-ci.yml
- confidence: cited
  path: deployment.lock.toml
- confidence: cited
  path: values.dev.yaml
revision: 5
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

## Delivery evidence

- Substrate release `0.7.0` was consumed from the owning repository by immutable daemon digest; Devcenter built no image and repackaged no chart.
- The downstream lock, environment value, and pipeline release variable changed only the Substrate selection.
- The downstream pipeline completed validation, deployment, and verification successfully.
- The Substrate StatefulSet retained its object identity, advanced exactly one generation, converged its current and update revisions, and reported one ready replica.
- Every unrelated Deployment and StatefulSet retained its pre-promotion UID, generation, and image digest.
- Product health and readiness probes both returned HTTP 200 after rollout.

This is the first affected-only operational slice. The broader story remains active until the private composition boundary can derive and reconcile every selected release unit from the ESS stack lock.
