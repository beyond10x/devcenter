---
format: aep.planning-md/1
id: story:local-k3d-acceptance
kind: story
status: implemented
title: Verify the composed engineer journey locally before publication
relations:
- decomposes: epic:independent-component-delivery
- informed_by: story:projects-connection-recovery
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: README.md
- confidence: cited
  path: b10x.docs.yaml
- confidence: cited
  path: ci/check-chart-rollouts.sh
- confidence: cited
  path: ci/local
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenterctl/Cargo.toml
- confidence: cited
  path: crates/devcenterctl/examples/local-node.rs
- confidence: cited
  path: crates/devcenterctl/examples/local-provider.rs
- confidence: cited
  path: crates/devcenterctl/src/deployment.rs
- confidence: cited
  path: crates/devcenterctl/src/lib.rs
- confidence: cited
  path: crates/devcenterctl/src/local.rs
- confidence: cited
  path: crates/devcenterctl/src/local_bootstrap.rs
- confidence: cited
  path: crates/devcenterctl/src/local_build.rs
- confidence: cited
  path: crates/devcenterctl/src/main.rs
- confidence: cited
  path: deploy/charts/devcenter/templates/substrate.yaml
- confidence: cited
  path: docs/local-acceptance.md
- confidence: cited
  path: frontend/acceptance
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/playwright.acceptance.config.ts
- confidence: cited
  path: frontend/src/features/workbench/devcenterWorkbenchHost.ts
- confidence: cited
  path: frontend/tests/devcenter-workbench-host.test.ts
- confidence: cited
  path: frontend/tsconfig.node.json
revision: 19
---
## Outcome

Run the actual Devcenter service composition in an isolated local k3d installation before publication, with locally built candidate images and pinned unchanged dependencies. Exercise the browser journey against real internal service boundaries and retain a usable local URL, component results and failure evidence. Reclaim obsolete build caches before building, without deleting source, persistent application data or evidence.

## Acceptance

- The Rust deployment CLI owns repeatable local preparation, image loading, chart composition, verification and owned-resource cleanup. Every Kubernetes invocation uses the explicitly selected local context; existing deployments remain untouched.
- The same Helm chart and immutable deployment-lock validation used by deployment select the local candidate images. Cached builds rebuild only selected components, with a complete pinned baseline retained.
- Local Identity issues real sessions and Connectors derives grants through normal flows. Test fixtures stay at external provider boundaries; browser API interception cannot satisfy acceptance.
- Headless browser checks cover fresh Git materialization, navigation, file edit/save/restore/reload, real terminal input/output and termination, followed by both Agent surfaces. A deterministic external model fixture is distinguished from live credential/provider acceptance.
- Representative history exercises pagination beyond an initial page. Previous Git HOME, paginated authority, and terminal-context regressions must fail their deciding local checks.
- Substrate execution profiles and storage prerequisites are proven on the selected local node before reporting terminal readiness; no unconfined application fallback can pass.
- Fresh and repeated runs retain logs, test results and exact artifacts; a failed or skipped component prevents an overall acceptance claim. Cleanup touches only resources owned by the local run.
- Documentation explains one repeatable entry point, local viewing, cold versus cached builds, deliberate cleanup and remaining differences from remote deployment.

## Existing seams and scope

The implementation builds selected application and composed Connector targets through the canonical Docker Bake graph, and optional Identity changes through its canonical Dockerfile. The actual chart composes Identity, Secrets, databases, Workspace, Substrate, Agent Platform and the BFF. New repository-owned headless browser tests exercise these actual service boundaries; external OIDC, forge and model endpoints are Rust fixtures.

The recorded scope follows the final diff. Rust CLI modules own build, preparation, composition, verification and cleanup. Two nonshipping Rust examples provide the external fixture and node storage prerequisite. The chart fixes the Substrate CA file mount, the composed Connector enables native CA roots, and the workbench waits for coordination admission before loading dependent layout and terminal state. Existing ESS workspace commands and deployment-lock semantics remain intact; there is no new product domain entity.

## Execution

User explicitly requested implementation and local k3d execution with disk reclamation. This is an interactive session. One owning story is used; no decomposition panel is needed for this single artifact.

## Current observations

The full local cycle passed after deleting and recreating the owned cluster and registry, then passed again on the same installation with the same project. The fresh node proved the real execution profile and parser, the project-quota filesystem, canonical candidate images, normal Identity login and Connector custody. Retained local image digests are republished before chart installation; protected profile verification files survive node replacement because the versioned AppArmor profile lives in the shared host kernel.

The deciding browser result includes Files entry and navigation, a materialized Git workspace, 44 non-overlapping visible editor lines, keyboard edit/save/exact restore/reload, binary PTY input/output with an executed random sentinel, acknowledged terminal termination, both agent replies, and acknowledged workspace plus coordination closure. The history check creates 40 closed coordination aggregates through the admitted BFF service API and follows ten real pages of four until a known record absent from the initial page is found. Repeated setup reuses its project and existing history. The external OIDC, forge and model endpoints remain explicit deterministic fixtures; this does not claim live model-provider acceptance.

Local testing exposed and fixed a workbench race: layout and terminal hydration could run before coordination resume was admitted. These dependent reads now wait for admission while Files remains usable. A delayed-admission unit regression and the existing progressive-entry browser check cover that ordering. The chart also mounts the Substrate Identity CA as the required regular file, and the composed Connector uses native CA roots without disabling verification. Administrative credential provisioning discovers declared requirements and applies a provider-neutral manifest through the normal API; GitLab is only fixture data.

The frontend gate, existing browser suite, root Rust gate, composed Connector gate, Identity gate, chart lint and rollout checks, version check, leak check and ESS validation passed. The existing browser suite reports 32 passes and 18 deliberately skipped mobile cases; the new composed acceptance checks skip none. The organization-wide documentation check remains refused by the current clean Atlas main because another repository already declares an unsupported documentation schema. This limitation is retained as a failure, not counted as green validation.

Obsolete Docker build layers and audited inactive Rust targets were reclaimed. Active caches remain for iteration; source, other deployments, persistent application data and private evidence were preserved. Public source contains no private baseline values, deployment credentials or browser sessions.
