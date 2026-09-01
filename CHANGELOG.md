# Changelog

## Unreleased

## 0.3.3 - 2026-09-01

- Roll Secrets whenever its rendered workload grants change, preventing the process from retaining
  a stale startup-loaded service-account subject after a Connectors identity transition.
- Verify that changing the configured Connectors service account changes the Secrets pod-template
  checksum before publishing the chart.

## 0.3.2 - 2026-09-01

- Canonicalize every chart resource name so a `devcenter` release no longer produces a second
  `devcenter` prefix on Services, ServiceAccounts, ConfigMaps, ingress, or policies.
- Allow a transition to retain an existing persistent-claim name and ensure an explicitly selected
  Connectors service account is also the exact subject granted Secrets workload authority.

## 0.3.1 - 2026-09-01

- Let cluster bootstrap own the exact Secrets TokenReview ClusterRoleBinding when deployment uses a
  deliberately namespaced reconciler, without granting that reconciler cluster-wide RBAC creation.
- Keep chart-managed TokenReview authority enabled by default for direct cluster-authorized installs.

## 0.3.0 - 2026-09-01

- Compose the public Secrets service directly in the Devcenter chart with digest-pinned images,
  Identity authority configuration, a read-only versioned keyring, and readiness probes.
- Project an exact-audience service-account token into Connectors and grant its exact subject only
  the tenant-scoped workload actions required by the remote secret-store adapter.
- Add optional single-replica PostgreSQL for development, external-DSN production configuration,
  persistent storage, and NetworkPolicy isolation allowing database access only from Secrets.
- Keep database credentials and key material in deployment-created Kubernetes Secrets; the public
  chart contains references and wiring only.

## 0.2.8 - 2026-09-01

- Reduce release-to-deployment latency by publishing the private Devcenter images for native
  `linux/amd64` only, sharing one cached builder across the server and deployment CLI targets, and
  scheduling the Devcenter workload explicitly onto compatible Linux AMD64 nodes.

## 0.2.7 - 2026-09-01

- Collapse repeated release/chart names for Deployment and pod workload names, so a `devcenter`
  release creates `devcenter` and `devcenter-<component>` workloads while retaining stable Service,
  configuration, service-account, and persistence resource names.

## 0.2.6 - 2026-09-01

- Default new agents to `claude-opus-5`, matching the current user-subscription model without
  relying on Claude Code's legacy-model remapping.
- Use the standard GitHub-hosted Intel macOS runner for x86_64 release bundles and let independent
  release targets finish when one matrix entry fails.

## 0.2.5 - 2026-09-01

- Replace the dashboard's pasted-credential journey with a Connector-owned Claude OAuth2 PKCE
  start and completion flow, including refresh-capable custody in Connectors.
- Keep the opaque connection flow only in browser memory and clear the one-time code immediately
  after submission. Provider tokens and PKCE verifiers never enter Devcenter persistence.
- Distinguish Agent Platform authentication refusals, operation refusals, invalid requests, and
  transport outages using browser-safe codes without relaying downstream response bodies.
- Record the architectural boundary that Identity remains provider- and service-agnostic.

## 0.2.4 - 2026-09-01

- Fix the upstream login callback by exchanging OAuth authorization codes as standard
  `application/x-www-form-urlencoded` data through Identity 0.3.2.

## 0.2.0 - 2026-09-01

- Add generic Identity-backed authorization-code login with S256 PKCE, one-use state, and an opaque
  secure browser session.
- Add an explicit BFF allowlist for session facts, Claude Code connector custody, Agent Platform
  agent management, Task submission and Task event streaming.
- Add the first authenticated engineer dashboard and expanded embedded OpenAPI/documentation.
- Extend the public Helm chart to compose digest-pinned Identity, Connectors and Agent Platform
  services behind internal ingress with deployment-owned configuration and persistent state.
- Keep application source and images private while retaining the configuration-neutral chart as
  the only public release artifact.

## 0.1.1 - 2026-09-01

- Require namespace creation to be explicitly requested by `devcenterctl apply`, allowing a
  namespace-scoped deployment identity to upgrade an operator-bootstrapped namespace.
- Prevent an in-tree confidential-marker denylist from matching its own contents.
- Verify package visibility through supported GitHub APIs during release.

## 0.1.0 - 2026-09-01

- Add the generic Devcenter HTTP service with embedded docs and OpenAPI.
- Add the Rust deployment and confidential-marker verification CLI.
- Add the public, configuration-neutral Devcenter Helm chart.
