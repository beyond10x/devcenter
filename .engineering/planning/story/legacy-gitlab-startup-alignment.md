---
format: aep.planning-md/1
id: story:legacy-gitlab-startup-alignment
kind: story
status: implemented
title: Align composition with safe legacy GitLab startup
relations:
- derived_from: epic:authenticated-control-plane
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: openapi.json
revision: 7
---
## Outcome

Consume the exact published compatible provider revision that leaves legacy GitLab connections unusable while keeping the host available for verified reconnection. Align the nominal Connector service factory types across Service SDK, generated AgentIDE and Todo, and the composed Devcenter service. This is a source-pin and release-identity update; the semantic service contracts remain unchanged.

## Acceptance

Regenerate owned outputs through their declared generator, refresh locks, and run the complete repository gate. Publish exact source coordinates for the provider-first deployment. Legacy credentials must not acquire a current grant from configuration; recovery requires the existing verified connect flow.

## Evidence

Devcenter 0.8.19 pins Connectors 0.6.5 at 235558c11f5fc2e4b8f8440474fb975df49d5329, Service SDK 0.5.11 at 0118bd3f9d63ead5d525fb39324b1e5e13c4ab1a, AgentIDE 0.3.7 at 176a57f58457a7c16f105584c66964263b3c2e41, and Todo 0.3.7 at dba6069cfcdd145a1bf25ea63f7762e1cc0b7e75. The provider allows startup with older GitLab metadata while leaving those connections unavailable until the normal verified reconnect; no grant is inferred from configuration.

The complete repository gate passed: frozen frontend install with pinned Node 24.20.0 and pnpm 11.25.0, frontend checks, Chromium E2E (19 passed; 15 existing platform-specific skips), formatting, all-target Clippy and tests for both Rust workspaces, version consistency, Helm lint, chart rollout checks, and leak check. The exact final composed source pins passed all three composition tests. Provider validation additionally passed the complete Connectors gate and 40 GitLab tests, including real Git v2 clone and verified legacy reconnection with preserved custody. Runtime and deployment publication is coordinated by Atlas; this source evidence does not claim a successful hosted rollout.
