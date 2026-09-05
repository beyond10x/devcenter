---
format: aep.planning-md/1
id: story:composed-tls-startup
kind: story
status: implemented
title: Initialize TLS before hosted background clients
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
  path: ci/check-chart-rollouts.sh
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: openapi.json
revision: 7
---
## Outcome

Devcenter 0.8.20 selects the composed Connector process's audited TLS cryptography provider before opening stores or restoring background clients. Optional listener configuration no longer determines whether those workers can construct HTTPS clients.

The chart separates the admitted HTTPS service port 443 from its unprivileged TLS listener on 8443 and scopes the network policy to the actual listener. Substrate receives its Git CA bundle through a regular-file subPath mount, satisfying its startup validation. The values describe the required rollout after CA Secret changes. Unsupported enabled service ports are refused during chart rendering.

## Evidence

The regression reproduced the actual implicit Rustls client-construction panic with the combined provider features before initialization. It passes after the startup initializer, before any optional listener is configured. All four composed-service tests, formatting and all-target Clippy passed. The full root Rust gate, pinned frontend checks and Chromium E2E (19 passed; 15 existing skips), version checks, Helm lint, chart rollout checks, and leak check passed.

Chart regressions check the 443 service to 8443 listener mapping, the regular-file CA mount and rejection of the unsupported HTTPS service port. Chart 0.8.20 accompanies the corrected provider; the existing server 0.8.18 and other immutable service artifacts remain reusable through independent publication.
