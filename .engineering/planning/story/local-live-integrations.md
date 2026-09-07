---
format: aep.planning-md/1
id: story:local-live-integrations
kind: story
status: active
title: Test real integrations in the retained local deployment
relations:
- informed_by: story:local-k3d-acceptance
- informed_by: story:agent-management-controls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenterctl/src/local.rs
- confidence: cited
  path: crates/devcenterctl/src/local_bootstrap.rs
- confidence: cited
  path: crates/devcenterctl/src/local_build.rs
- confidence: cited
  path: crates/devcenterctl/src/local_integrations.rs
- confidence: cited
  path: docs/local-acceptance.md
- confidence: cited
  path: frontend/acceptance
- confidence: cited
  path: frontend/src/features/connections
- confidence: cited
  path: frontend/src/features/connectors
revision: 12
---
## Outcome

The retained local deployment exercises real activated provider integrations before publication. Private deployment configuration supplies the repository forge, collaboration provider and observability origin; generic source owns no organization-specific identifiers.

## Acceptance

- Explicit live mode preserves the private provider configuration and normal Identity, Connector custody and grants. Fixture mode is recorded separately from the always-real model mode.
- Generic setup uses declared administrative requirements or owner-scoped Connect Sessions. Provider endpoint configuration alone never manufactures a credential or connection.
- Credential values stay in normal custody or owner-only input files, never source, command arguments, environment variables, logs or evidence.
- Actual freshly described read operations succeed for every configured provider, and the real repository materializes with working Files and terminal before complete acceptance.
- Only changed components are built; normal iterations retain the cluster, databases, model connection, CA, quota state and caches.

## Implementation

The local Rust CLI retains live/fixture selection, clears inherited fixture provisioning on mode changes, supports a local Agent Platform source build, and invalidates acceptance receipts on prepare/apply/test. A live test refuses a fixture-derived last project. Required provider reads derive from composed private configuration and use fresh admitted operation descriptions.

Connectors endpoint and form candidate 129aece1ef6ae3717229cf1505a40106cdee92c1 is composed and deployed locally. The hosted catalog form renders the selected profile's credential label, help and documentation, including the observability service-account-token versus SSO-password distinction. Native administrative requirements remain distinct from generic catalog principal-owned credentials.

## Verification

The full required Devcenter source gate passed, including frontend checks, browser regressions, root and composed Connector Rust checks, generated contract consistency, chart, version and leak checks. The upstream endpoint/form candidate passed its full gate. Actual headless browser checks passed for all three provider forms, including the declared observability destination, token label, SSO guidance and official documentation link. Actual agent and capability-profile lifecycle checks passed against the retained local composition.

All three real external provider reads currently return authorization required. Service-account provisioning has not been located through the available integration access and the operator has been asked for its reference. Real repository materialization and PTY acceptance remain pending normal Git authorization; the retained fixture-derived project cannot satisfy live acceptance. No successful external read is claimed from a form, URL binding or custody status.

The latest real conversation test remains incomplete after a properly typed history correction: the provider still refused the recall turn with category reasoning_extraction. This is recorded as a failure rather than hidden by partial output or replaced by the earlier successful run.

## Programmatic setup follow-up

The generic hosted token CLI/client candidate reuses the existing owner-scoped Connect Session. It accepts an owner-only token file, validates the exact completion destination, submits once without redirects or automatic retry, and confirms owner-scoped status and connection description. The CLI checks its catalog declaration before reading credentials or opening Identity state. Its supported scope is single-secret catalog token profiles; OAuth consent and native multi-field acquisition keep their normal routes. The reusable protocol client remains independent of catalog implementation and supports normally issued short-lived Identity authority without a desktop keyring.

The candidate passed focused client and CLI boundary tests and all-target clippy. Native CA support was verified with TLS tests and then through actual normal Identity login against the retained local deployment. A fresh headless browser completed upstream login and the loopback callback; the CLI stored its session through the normal OS keyring and selected the local deployment in an isolated XDG state directory. A subsequent owner-scoped hosted connection list succeeded. The ordinary CLI metadata still selects the existing non-local deployment. The first browser helper attempt failed before browser startup because its TMPDIR was not set; after correcting that environment prerequisite, the normal login passed. Failed evidence is retained.

The Grafana operation search still returned no admitted operations, and no provider token has been supplied. Thus actual token acquisition and external reads remain unverified. The candidate is published at acf8ef8c01e6a794860babd5d6df15a2d71b8014. Its full repository gate passed all twelve workspaces, both runtime feature configurations, catalog and documentation checks, and exact ESS projection. Client, CLI and console all-target clippy passed. The tested CLI binary matches the final implementation; only evidence updates followed its build. This is a client-only candidate compatible with the deployed endpoint/form runtime; no service image rebuild or shared deployment was performed.
