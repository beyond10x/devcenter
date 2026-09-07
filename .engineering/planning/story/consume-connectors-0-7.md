---
format: aep.planning-md/1
id: story:consume-connectors-0-7
kind: story
status: active
title: Use released Connectors 0.7 across the Devcenter composition
relations:
- decomposes: epic:independent-component-delivery
- informed_by: story:refresh-user-bound-model-credential
- informed_by: story:live-model-local-acceptance
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenter-http/src/lib.rs
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/api/schema.gen.ts
- confidence: cited
  path: frontend/tests/client.test.ts
- confidence: cited
  path: openapi.json
revision: 9
---
## Outcome

Consume the operator-selected released Connectors 0.7.0 contract in the BFF client and composed runtime, including the required generated service SDK alignment. The release tag resolves to e80b7ae1b2151d13aa9786cf67ea05e66717ee35. The OAuth recovery patch must remain present on this release baseline; do not regress its distinctions or keep the older compatibility baseline as the final solution.

## Acceptance

- Root Connector client/protocol and composed runtime/service/protocol use the 0.7 release contract at immutable source revisions. The task's engineering integration CLI also uses the verified official 0.7.0 binary.
- Align generated service consumers with the 0.7 OperationDescription rate advice field through the owning service SDK, preserving truthful absence of advisory metadata when none is declared. No vendored or temporary source patch may substitute for an upstream SDK correction.
- Resolve actual compiler and contract failures locally, retain frozen dependency resolution and generic source/privacy boundaries, then build the actual Docker composition in k3d.
- The user-owned model connection is preserved. Real Claude replies, Files and PTY must pass locally before remote publication and promotion. Missing authorization remains an explicit blocker.

## Scope

Cited: Cargo.toml and Cargo.lock pin the root Connector client and protocol. crates/devcenter-connectors/Cargo.toml and Cargo.lock pin runtime, service, protocol and generated SDK adapters. The service SDK owns the affected generated Connector and catalog factories. No new product entity is introduced by this release migration.

## Authorization

The operator explicitly requested https://github.com/beyond10x/connectors/releases/tag/v0.7.0 and directed that it be used. Advance this single bounded implementation from draft through proposed to active; no decomposition panel is needed for a single story.

## Observed root migration requirements

Local compilation of the released client found its default operation response now carries v0alpha3 errors. Devcenter must consume the typed error and map the additive authentication-required and rate-limited outcomes explicitly. Preserve a trusted retry delay as an HTTP Retry-After header and show an actionable connection message without turning provider authentication into a Devcenter session expiry. These are release-contract changes discovered before container build or deployment.

## Published dependency validation

The BFF consumes release commit e80b7ae1b2151d13aa9786cf67ea05e66717ee35. The composed runtime consumes 1c45d5bfbdc583738e90f2b93cf9c124ed2c5568, the same release baseline with the OAuth failure recovery patch. The upstream SDK correction is published at 2cc5d56c694b9081907c37d7c41cc404cbbe4bdd and passed its complete repository gate, including the real PostgreSQL persistence workload. The composition unifies the generated applications on this SDK without modifying their generated source, and aligns Eventlog at 081815cdfcbf1c751e7ee91abd81af2ff7d460cb to preserve Rust trait identity.

Final dependency resolution uses published Git revisions only; temporary local SDK patch configuration is excluded. Locked composed checks, clippy and tests pass. Root workspace checks and tests pass, including 31 BFF tests. Frontend verification passes 52 unit tests and 36 browser tests, with 18 existing applicability skips. Format, version consistency, chart lint, eight volume permission execution cases and confidential-marker checks pass. The organization documentation check remains refused by an unrelated repository manifest schema unsupported by current Atlas.

This is source-level evidence. The actual 0.7 containers must still be installed and tested in k3d. Full live-model acceptance and remote promotion remain pending user-owned model authorization; no synthetic credential or mocked model result may satisfy that gate.
