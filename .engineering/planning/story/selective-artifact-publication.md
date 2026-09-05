---
format: aep.planning-md/1
id: story:selective-artifact-publication
kind: story
status: active
title: Publish only affected Devcenter artifacts
summary: Separate server, deployment CLI, Connectors runtime and chart publication with independently selected immutable outputs.
relations:
- decomposes: epic:independent-component-delivery
scope:
- confidence: cited
  path: .github/workflows/gate.yml
- confidence: cited
  path: .github/workflows/promote-connectors.yml
- confidence: cited
  path: .github/workflows/release.yml
- confidence: cited
  path: ci/check-release-unit-impact.sh
- confidence: cited
  path: ci/check-version-consistency.sh
- confidence: cited
  path: ci/release-unit-impact.sh
- confidence: cited
  path: crates/devcenterctl/src/lib.rs
- confidence: cited
  path: crates/devcenterctl/src/main.rs
- confidence: inferred
  path: crates/devcenterctl/src/publication.rs
revision: 6
---
## Outcome

Publish Devcenter release outputs independently, serving O4 from AGENTS.md and the existing independent-component-delivery epic.

## Context

The four output identities already exist in ess/build.yaml. The unconditional images and chart jobs in .github/workflows/release.yml ignore ci/release-unit-impact.sh; ci/check-version-consistency.sh couples chart and Connectors versions to the server. The existing impact-aware-oci-gate story explicitly excludes publication.

## Acceptance

An automatic or explicitly selected publication builds only affected server, deployment-cli, connectors or chart outputs, skips unchanged outputs before allocating build runners, and publishes durable immutable metadata preserving every reused artifact's original version, source commit and digest.

Required executable cases: frontend-only, CLI-only, Connectors-only, chart-only, docs/no-op, release-version-only churn, real dependency change, missing first baseline, failed/incomplete publication, staggered per-artifact baselines, and immutable reuse. Chart publication has no image-build prerequisite. Selection compares each candidate with its own last successful publication; explicitly selecting one must not erase pending changes for other outputs. Malformed or unavailable baseline history refuses rather than silently rebuilding all. Existing digest-only manifest consumers remain compatible or receive a documented explicit migration.

Separate workflow dispatch selections supply independent triggers; tag publication automatically selects affected outputs. Version checks must stop requiring independent chart and Connectors versions to equal the server. Build and publication logic reuse the existing ESS output identities and generated build graph. Runtime/orchestration logic is Rust; CI adapters may invoke tools.

## Scope

Derived 2026-09-05 by aep-drive:story-scoper. Primary surfaces .github/workflows/release.yml, .github/workflows/gate.yml, ci/release-unit-impact.sh, ci/check-release-unit-impact.sh, ci/check-version-consistency.sh and crates/devcenterctl/src/{main,lib}.rs are cited. New publication.rs and dependency manifest edits are inferred. Existing promote-connectors.yml is cited and must route through the same publication policy rather than retain a second bypass.

Confidence high, because current entrypoints and ESS output identities were read. Collides with release workflow and devcenterctl edits. Conceptual documentation is coordinator-owned.

## Out of scope

New application features, dependency protocol migrations, cluster configuration and new ESS format semantics.
