# Changelog

## 0.5.10 - 2026-09-02

- Make the complete Connector catalog the default `/connectors` workspace, with bounded provider
  search, provider and operation detail, and setup actions derived only from runtime-advertised
  profiles; redirect the former `/connections` route and retire its hard-coded GitLab setup card.
- Pin Connectors 0.4.4 for the typed hosted catalog contract and complete GitLab setup profiles.
- Let deployments explicitly expose the Connector-owned docs and OpenAPI routes as exact ingress
  paths, defaulting them off and advertising them in the session only when they are reachable.

## 0.5.9 - 2026-09-02

- Compose Connectors 0.4.3 so hosted operators can inspect Integration readiness and supply
  credential values through the Identity-protected administration API.
- Expose only the `/api/connectors/v1/admin` prefix as a separately enabled ingress route, leaving
  Connectors responsible for its exact administrative audience, scope, and operator-group checks.

## 0.5.8 - 2026-09-02

- Publish executable Todo lifetime and scope shapes to agents, including the required future RFC
  3339 expiry, instead of exposing opaque type titles that invite invalid guesses.
- Report malformed or elapsed bounded lifetimes as invalid input after approval rather than the
  misleading `NotGranted` authorization refusal.

## 0.5.7 - 2026-09-02

- Admit the exact Identity authority used to issue Connector approval proofs, so an approved
  effect-bearing agent call reaches the composed Connector instead of failing at token verification.

## 0.5.6 - 2026-09-02

- Compose Todo service generation with Connectors 0.4.1 so the final Devcenter Connector retains
  deployment-declared, connection-bound post-DNS egress enforcement.

## 0.5.5 - 2026-09-02

- Publish the composed Connector runtime as the distinct `connectors-<version>` artifact in
  Devcenter's existing private package, after the release guard correctly refused a newly created
  package that inherited public visibility from the source repository.

## 0.5.4 - 2026-09-02

- Let signed-in people inspect and resolve exact agent Connector calls while the BFF keeps
  short-lived approval authority and one-use evidence out of the browser.
- Ship a dedicated Devcenter Connector composition which registers the generated Todo factory,
  applies only deployment-owned operation policy, and persists service events through Eventlog's
  PostgreSQL adapter.

## 0.5.3 - 2026-09-02

- Route public Connector setup and GitLab OAuth callbacks beneath the hosted Connector base path,
  matching the Connector's same-origin validation and internal API contract.

## 0.5.2 - 2026-09-02

- Capture Helm's complete OCI push output before extracting the immutable chart digest, including
  Helm versions that report a successful push on standard error.

## 0.5.1 - 2026-09-02

- Authenticate private Cargo dependencies while validating the complete release candidate, so a
  successful image build cannot stall chart and release publication at the final validation seam.

## 0.5.0 - 2026-09-02

- Show every Connector connection visible to the current Identity session and support GitLab OAuth
  or personal-token setup without placing credential bytes in Devcenter.
- Add governed capability profiles backed by exact, credential-free Connector operation snapshots,
  and bind new agents to a selected profile.
- Compose the released Workspace service for grant-derived repository discovery, exact default
  branch handling, project chat, workflow admission, central AEP navigation, and authenticated
  Substrate HTTPS access.
- Preserve structured model-execution failures and make durable Agent Platform and Workspace state
  available through generic chart persistence.
- Validate deployment locks, chart renders, immutable component pins, and provider-reported default
  branch release heads before publication or mutation, with bounded rollout diagnostics and timeout.

## 0.4.7 - 2026-09-01

- Keep Identity and publication storage mandatory for readiness while allowing explicitly absent
  optional service clients to remain disabled and fail closed at their route boundaries.

## 0.4.6 - 2026-09-01

- Probe Connectors beneath its configured hosted base path so Kubernetes observes the same live
  and ready endpoints exposed by the composed service.

## 0.4.5 - 2026-09-01

- Make Substrate TLS identity staging retry-safe and hand off child files before restricting the
  parent directory, retaining the initializer's narrow capability set.

## 0.4.4 - 2026-09-01

- Materialize Kubernetes-projected TLS identity entries as regular, owner-controlled files before
  starting Substrate, preserving its refusal to follow certificate or private-key symlinks.

## 0.4.3 - 2026-09-01

- Initialize Substrate state and runtime mounts as owner-only directories before starting the
  non-root daemon.

## 0.4.2 - 2026-09-01

- Repair PostgreSQL publication persistence, make terminal revocation atomic, and exercise the
  complete store contract against PostgreSQL in CI.
- Require hosted database configuration and add resource and health contracts for every composed
  chart workload.
- Hide publication existence until MCP authentication, accept standard OAuth callback parameters,
  and align the browser, OpenAPI, review mode, container example, and release-version checks.
- Add repository project browsing, snapshot-bound agent conversations, workflow launching, and
  central AEP artifact navigation without guessing the forge's configured default branch.
- Deploy Substrate as a persistent HTTPS service with explicit Identity trust, server identity,
  and certificate mounts, and connect Workspace through its released authenticated SDK contract.

## 0.4.1 - 2026-09-01

- Replace native release archives and the repeated tag gate with concurrent native-runner
  `linux/amd64` and `linux/arm64` container builds, signed multi-arch manifests, and the OCI chart.

## 0.4.0 - 2026-09-01

- Add provider-neutral Identity entry with opaque configured-provider selection, explicit browser
  logout semantics, and immutable agent revisions that can retain a capability profile.
- Add durable SQLite and PostgreSQL persistence for MCP publications, immutable revisions, client
  authorizations, and exact-input approval retries with CAS and terminal revocation semantics.
- Add Streamable HTTP MCP support for protocol versions `2025-11-25` and `2025-06-18`, including
  deterministic direct-tool projection, effect annotations, path-specific OAuth resource metadata,
  and stable publication endpoints.
- Add the Vue publication control surface, Codex and Claude Code setup guidance, configured Identity
  provider selection, and Secret-referenced hosted database configuration in the Helm chart.
- Keep production publication mutation and MCP invocation fail closed until Identity, Connectors,
  and Agent Platform expose the required resource-bound exchange, grant, approval, and profile APIs.

## 0.3.4 - 2026-09-01

- Replace the inline browser page with a dedicated Vue 3 and TypeScript application for Identity
  sessions, Connector custody, agent management, Task submission, streaming output, and embedded
  product documentation.
- Embed the deterministic Vite production bundle in the Rust server, serve only explicit SPA and
  asset routes, and remove inline-script and inline-style allowances from the application CSP.
- Add responsive operator-console navigation, complete loading/error/empty states, accessible
  workflows, frontend contract generation, and automated frontend gates.
- Add a visibly marked, process-local review mode for exercising the product journeys before a
  merge or deployment without contacting real services.

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
