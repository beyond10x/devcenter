---
format: aep.planning-md/1
id: story:ess-release-model
kind: story
status: active
title: Make ESS the canonical Devcenter release model
summary: Generate build execution and documentation from one typed multi-output ESS DAG, then release it.
relations:
- decomposes: epic:independent-component-delivery
scope:
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Dockerfile.ess
- confidence: cited
  path: ci/check-ess-model.sh
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: deploy/charts/devcenter/Chart.yaml
- confidence: cited
  path: docker-bake.hcl
- confidence: cited
  path: docs/ess-deployment-model.md
- confidence: cited
  path: ess/build.yaml
- confidence: cited
  path: ess/system
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: generated/ess
- confidence: cited
  path: openapi.json
revision: 4
---
## Context

Devcenter's release inputs previously existed as loosely related workflow and chart conventions. The
repository now has `ess/build.yaml`, generated BuildKit input, compiled IR, and a generated Mermaid
graph embedded in its own deployment-model documentation.

## Acceptance

Devcenter 0.8.6 is released from a green gate with its BuildKit inputs and documented concrete DAG byte-derived from the validated ESS build model, including isolated concurrent Cargo caches.

## Scope

- `ess/build.yaml` and `ess/system/`
- `generated/ess/`, `Dockerfile.ess`, and `docker-bake.hcl`
- `ci/check-ess-model.sh` and release gates
- `docs/ess-deployment-model.md`
- Devcenter 0.8.6 release metadata
