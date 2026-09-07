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
revision: 8
---
## Outcome

The retained local deployment can exercise real activated provider integrations, including the repository forge and collaboration and observability providers, before publication.

## Acceptance

- An explicit local provider mode preserves private baseline integration configuration and uses local Identity, Connector custody, and normal user connection flows.
- Generic operator credential provisioning remains provider-neutral and reads owner-only value files; no secret values enter source, arguments, environment variables, logs, or evidence.
- Fixture setup cannot overwrite live provider configuration or credentials on subsequent local builds or browser retries.
- Acceptance records the provider mode and validates real repository discovery, materialization, and an admitted read operation for each configured provider; missing user authorization remains a visible incomplete result.
- Only changed services are built, no release is required to test them, and retained databases, model connection, CA, caches and cluster identity survive ordinary iterations.
- The browser exposes normal connection setup and explains actionable prerequisites for integrations awaiting authorization.

## Scope

The local deployment CLI and acceptance runner, generic configuration and credential setup, Connections UI where a diagnosed omission prevents setup, and reusable local-development documentation. Provider endpoints and credentials remain in private local inputs. No remote release or provider write operation is required for these read-only checks.

## Local verification

The generic hosted-endpoint candidate is composed from Connectors 7e11cb919bd2746e8d3e04c1e74ede788409f147. Its complete upstream gate and the consuming Connector fmt/clippy/tests pass. Actual local browser setup forms pass for the three requested providers, including the private deployment's bound observability destination. The reusable integration suite derives configured provider targets from the composed values and invokes only freshly described admitted read operations. Its latest real run reaches Identity and Connector search successfully, then reports authorization required for all three providers. Provider credential entry remains pending owner setup; there is no successful real repository or provider read yet. The receipt remains incomplete and the fixture project cannot be reused as live acceptance. No shared deployment or release was performed.

## Current local verification

The local CLI now retains explicit fixture/live selection, preserves private provider declarations in live mode, clears inherited fixture provisioning, supports a local Agent Platform source build and invalidates earlier acceptance receipts on prepare/apply/test. A live test refuses a fixture-derived last project. Required provider reads derive from the composed configuration, obtain fresh Connector descriptions, and fail on missing authorization.

Full source checks passed: frontend formatting/lint/types/unit/build, generated API consistency, browser regression, root Rust fmt/clippy/tests, composed Connector fmt/clippy/tests, chart checks, version and leak checks. Actual local agent/profile lifecycle checks passed, and the provider forms opened through the real Connector. The generic catalog form now consumes credential-specific guidance from upstream candidate 129aece1ef6ae3717229cf1505a40106cdee92c1, whose full upstream gate passed. Browser verification of that latest form image is pending.

Actual reads from all configured external providers remain blocked by missing normal authorization. Generic catalog service-account credentials currently have only the principal-owned Connect Session path; the existing administrative provisioning endpoint cannot provision them. This is an explicit remaining gap for deployment-managed setup, not a credential supplied by a URL binding. Real model acceptance also remains incomplete: a complete conversation run passed once, but a later composed run received a terminal provider refusal with partial output. The separately gated provider-diagnostic candidate is being composed; no refusal has been suppressed or counted as success. No release or shared-environment promotion is ready.
