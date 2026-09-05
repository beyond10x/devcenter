---
format: aep.planning-md/1
id: story:faster-coding-workspace
kind: story
status: implemented
title: Open coding workspaces progressively
tags:
- coding-workspace-speed
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
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenter-http/src/lib.rs
- confidence: cited
  path: frontend
- confidence: cited
  path: openapi.json
revision: 9
---
## Outcome

Implemented progressive coding-workspace startup and integrated the Git Smart HTTP v2 providers through exact published source commits. The lightweight route validates project and session concurrently while renderer loading overlaps session creation. Once materialization is ready, the file tree and focused editor are available independently of remaining saved files, coordination and terminals. Preparing sessions poll automatically with bounded retry backoff and cancellation.

## Scope

The browser startup, host adapter, renderer entry and targeted regression tests; BFF coordination reads and handler timings; application version 0.8.18 and exact Cargo dependency pins, including the separate composed Connectors workspace. Generated service type compatibility requires Service SDK 0.5.10, AgentIDE 0.3.6 and Todo 0.3.6 source candidates. The generic chart has no configuration change.

## Acceptance

Authorized files are available without waiting for unrelated startup services. Navigation aborts pending work. Delayed layouts and file responses preserve edits, closures, cursor state, focus and newer acknowledged layout versions. Restore the focused file first and limit remaining restoration to four concurrent requests. Mutations stay ordered and retries retain their idempotency identity. Every remote operation retains BFF authorization. Preserve materialization depth 50 through the exact Connectors, Workspace and Substrate source chain. Source publication is distinct from release and hosted rollout.

## Evidence

The frontend gate passed: frozen pnpm installation, formatting/lint, API generation, typecheck, 46 unit tests and production build. Chromium Playwright completed with 19 passing tests and 15 existing platform-specific skips. The workspace route is 32.60 kB uncompressed and 10.57 kB gzip; the renderer loads separately. Regressions cover delayed layout/coordination/terminal replies, automatic preparation polling, bounded file restoration, cancellation, stale data, edits and pane actions. Browser startup marks use stable names without project or actor identifiers.

The root Rust formatting, all-target Clippy with warnings denied and workspace tests passed with the final Connectors and Workspace pins. The PostgreSQL store contract also ran against an isolated PostgreSQL 17 fixture and passed. The separate composed Connectors workspace passed formatting, all-target Clippy and all three tests with its final published pins; its production dependency tree has one SDK 0.5.10 source and one Connectors 0.6.4 source. Version consistency, Helm lint/rollouts, release-unit impact checks, leak checks and ESS model/build fixed-point validation passed. ESS used the repository-pinned compiler 0.9.2.

Provider sources are Connectors dbdd285c629d8b93bb685cc5a89a270316978ce5, Substrate 329a128606a7c18c4477d6fff58ffc57b296fb70, Workspace f348e3f3e6302e81b09cd4eda1d70affbe24a392, Service SDK 14bb3ba65b3f0f2b8a577bd9f30961bfc0a92ad9, AgentIDE e03f8578388ce7e3a779007fb5c7a814c43afdd5 and Todo 2263d9adec22f64db2563a4f731b2b3f275cc23e. Every owning repository passed its full local implementation gate. Real HTTPS Git tests verify protocol v2 on both sides of the scoped proxy and exact 50-commit materialization; controlled many-ref discovery traffic fell by more than 99.8 percent. This is fixture evidence, not a hosted click-to-editor measurement.

The clean current-main Atlas documentation check independently encounters existing AgentIDE v4 manifest incompatibility in the pinned Docs System collector. Atlas's wider fence also reports the existing Website Docs System pin mismatch and ungrounded Widgets map entry. These unrelated delivery issues were recorded without rewriting those repositories. No production deployment or hosted speed improvement is claimed; rollout still requires the deployment target and source/release review.

The exact generated Dockerfile built all three affected OCI targets for linux/amd64 with `docker buildx bake --file docker-bake.hcl server connectors deployment-cli --load`. The Devcenter server passed the documented nonroot/writable-state-directory startup and HTTP health smoke; Connectors and the deployment CLI ran with networking disabled and reported 0.8.18. Build and smoke checks exited 0. The local Docker credential configuration initially refused a public base-image pull; an isolated empty authentication configuration allowed the exact immutable base to build without changing any repository input. Images are retained locally as devcenter:coding-v2-gate (sha256:8f4cf5c7c6fbf71570603437c2ab4125bbf11a74a7517cb26c696e519d84bcfa), devcenter-connectors:coding-v2-gate (sha256:9849ef3779915608ac018d3a9a3d33607e1f23daae85720ca1862d9ca643b7cd), and devcenterctl:coding-v2-gate (sha256:fa05de39411b9c80850983b743bda0f1941913048f7103b7d04fac629457bc56). These are local validation artifacts, not published production images.
