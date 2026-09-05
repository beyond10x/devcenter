# Devcenter

Devcenter is a generic control surface for governed agents, workflows, connectors, and sandboxed
execution. This public repository contains the application source and deployment CLI, plus the
source of the public, configuration-neutral Helm chart. The
[PolyForm Free Trial License 1.0.0](LICENSE) grants a trial of less than 32 consecutive days to
evaluate Devcenter for a particular application; production, redistribution, and post-trial use
require a separate agreement with beyond10x. Application and deployment CLI container images remain
private. Devcenter does not publish native binary archives.

Choose the path that matches what you are trying to do:

- [Review the frontend locally](docs/frontend-review.md) — public, credential-free evaluation.
- [Build the full service from source](docs/source-build.md) — preview access for approved evaluators.
- [Understand the ESS deployment model](docs/ess-deployment-model.md) — how independently released
  systems compile into one exact, reviewable deployment.
- [Check production deployment availability](docs/production-deployment.md) — currently paused and
  dependent on private artifacts and deployment inputs.

The repository deliberately contains no real deployment values. A deployment supplies its tenant,
hosts, image mirrors, Identity configuration, generated-service deployment overlays, connector
catalogue bundle, storage classes, and Secret references separately.

## Run locally

The quickest public review needs no service credentials or deployment. It runs every frontend
journey against process-local sample data and is visibly marked as review mode:

```bash
npm install --global pnpm@11.25.0
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend review
```

Open `http://127.0.0.1:4173`. Review mode contacts no Identity, Connector, model provider, or Agent
Platform. This source-only path is the public evaluation path and needs no beyond10x account or
private artifact.

### Run the full service container

This path is available only to evaluators with approved access to the private sibling dependencies.
The public review mode above does not require that access.

Build the same Linux containers used by deployments. `ess/build.yaml` is the canonical build graph;
the committed `Dockerfile.ess` and `docker-bake.hcl` are deterministic projections checked by CI.
The token lets package managers fetch approved GitHub-hosted dependencies through HTTPS and is not
retained in an image layer:

```bash
DEV_CENTER_BUILD_TOKEN="$(gh auth token)"
docker buildx bake --file docker-bake.hcl \
  server connectors deployment-cli \
  --load \
  --set '*.platform=linux/amd64' \
  --set '*.secrets=id=github-token,env=DEV_CENTER_BUILD_TOKEN' \
  --set 'server.tags=devcenter:local' \
  --set 'connectors.tags=devcenter-connectors:local' \
  --set 'deployment-cli.tags=devcenterctl:local'
```

One Bake invocation shares the frontend and Rust build nodes across outputs. Run
`bash ci/check-ess-model.sh` with ESS 0.9.1 installed to prove that the committed projections still
match the typed semantic and build sources.

```bash
docker run --rm --publish 8080:8080 \
  --env DEV_CENTER_LISTEN=0.0.0.0:8080 \
  --env DEV_CENTER_TENANT_ID=local \
  --env DEV_CENTER_PUBLIC_ORIGIN=http://127.0.0.1:8080 \
  devcenter:local
```

Open `http://127.0.0.1:8080/docs/` for the embedded documentation and
`http://127.0.0.1:8080/openapi.json` for the service contract.

For frontend development, run `pnpm --dir frontend dev`; Vite proxies the allowlisted API and auth
routes to the container on `127.0.0.1:8080`.

Protected routes fail closed until an Identity verifier is configured. For loopback-only MCP
development, also set `DEV_CENTER_INSECURE_DEV_AUTH=true` and a non-empty
`DEV_CENTER_DEV_BEARER_TOKEN`. Local publication records use `devcenter.sqlite`; override
`DEV_CENTER_DATABASE_URL` with another SQLite URL when isolation is useful.

In a production posture, set the Identity origin and exact web callback, plus the private inner
service origins:

```bash
DEV_CENTER_IDENTITY_ORIGIN=https://identity.example.test \
DEV_CENTER_IDENTITY_AUDIENCE=urn:b10x:devcenter \
DEV_CENTER_IDENTITY_WEB_CLIENT_ID=devcenter-web \
DEV_CENTER_IDENTITY_REDIRECT_URI=https://devcenter.example.test/auth/sso/callback \
DEV_CENTER_IDENTITY_PROVIDERS='[{"id":"provider-a","display_name":"Provider A"}]' \
DEV_CENTER_IDENTITY_EXCHANGE_CALLER_ID=relying-service \
DEV_CENTER_DATABASE_URL=postgresql://... \
DEV_CENTER_AGENT_PLATFORM_ORIGIN=https://agents.example.test \
DEV_CENTER_CONNECTORS_API_BASE=https://connectors.example.test/api/connectors/v1 \
DEV_CENTER_WORKSPACE_ORIGIN=https://workspace.example.test \
DEV_CENTER_WORKFLOW_ORIGIN=https://workflow.example.test \
DEV_CENTER_AGENTIDE_WORKSPACE_ENABLED=false
```

The matching `DEV_CENTER_IDENTITY_EXCHANGE_SECRET` is injected from deployment-owned Secret
material, never a ConfigMap or public values file. Identity admits that caller only under an exact
source-audience, source-scope, target-audience, and target-scope exchange policy.

`DEV_CENTER_WORKFLOW_ORIGIN` enables the standalone Workflow library. Devcenter exchanges the
browser session for only `workflows.read` at `urn:b10x:workflow`; project-bound workflow runs remain
served by Workspace.

The browser receives only an opaque, Secure, HttpOnly session cookie. `Connect Claude` starts a
Connector-owned OAuth2 PKCE flow: Devcenter retains only an opaque flow id in browser memory while
the user authorizes, and Connectors owns provider exchange, refresh, and credential custody. Agent
Platform receives an attempt-bound lease and Harness redeems it only at the provider request
boundary. Agent Connector calls use separate attempt-bound invoke authority. If a call requires a
human decision, the BFF obtains short-lived approval authority, asks Connectors to seal the exact
operation, connection, description lease, and input, and hands the one-use proof directly to Agent
Platform. Neither credential reaches the browser. Identity remains provider- and service-agnostic
throughout.

The generated-services workspace uses the ordinary Connector composition seam: Todo and AgentIDE
contribute their generated domain factories, while Service SDK contributes a separate external
catalog factory containing both exact `service-catalog/1` contracts. Both generated services use
the composition's existing Eventlog PostgreSQL adapter. Devcenter does not implement a parallel
AgentIDE database, event bus, or service runtime. Devcenter's BFF reads that catalog, verifies
every requested operation against it, and keeps Connector descriptions, connection selection,
ephemeral access tokens, and approval evidence server-side. The browser imports the exact
`@b10x/service-console-vue` package used by each synthesized service's standalone generated docs;
only the binding changes from disposable demo mode to the authenticated live endpoint. Tenant and
user are login facts, the Devcenter realm is absent (`None`), and no realm selector appears in a
route, request body, header, or browser-client argument.

The native coding workbench is available at `/projects/:projectId/sessions/:sessionId` only when
`DEV_CENTER_AGENTIDE_WORKSPACE_ENABLED=true`; the public chart defaults it off. Its file tree,
complete file reads, digest-guarded writes, immutable-base comparison, and canonical diff
projections all pass through Devcenter's existing authenticated Workspace client. Workspace remains
the sole authority for project materialization and delegates filesystem execution to Substrate.
AgentIDE stores only coordination state—session binding, bounded grants, context references and
digests, and approval checkpoints—through its generated Service SDK/Eventlog service. Unsaved editor
buffers and selected source bytes remain in the browser until an explicit save or attachment action.
No host shell fallback exists: without a deployment-declared Substrate terminal profile and an
explicit grant, the terminal pane renders a refusal. With both authorities present, Devcenter
lazily loads its vendored `ghostty-web` renderer and attaches through a same-origin WebSocket BFF;
the BFF carries the current Identity authority to Workspace in a header and never exposes it in a
URL. Binary input and sequenced PTY output remain byte-exact, lifecycle and replay bounds remain
explicit, browser tab close only detaches, and the separate Kill action terminates the Substrate
process. The terminal row is keyboard- and pointer-resizable, while scrollback enters AgentIDE
context only when a human explicitly attaches a selection.

Git coding sessions request hard byte and inode quotas. Their Substrate host must prove those
guarantees before admitting a materialization. The chart's optional
`substrate.workspaceStorage.existingClaim` mounts an operator-provisioned workspace filesystem
separately from durable service state. Setting `workspaceStorage.projectQuotas.enabled` and its
inclusive `idsStart`/`idsEnd` range delegates at least 128 exclusive project IDs to that host.
The filesystem must enforce project quotas; the released runtime proves inheritance, accounting,
byte enforcement and inode enforcement at startup. An ordinary volume or directory-size check
does not provide that guarantee.

This option explicitly grants the non-root daemon `SYS_ADMIN`, which Linux requires for project
quota management. Kubernetes consequently enables privilege escalation for that container. All
other capabilities remain dropped, the root filesystem remains read-only, and the chart adds no
host mounts or host namespaces. The default leaves this authority disabled. The operator must
verify that the selected runtime's child processes cannot inherit that capability; enabling a
terminal execution profile additionally requires its existing sandbox and cgroup guarantees.

For an existing installation, pause workspace writers before copying the complete workspace tree,
including hidden baseline data, onto the new filesystem. Verify file contents and metadata before
switching mounts; keep the original state volume and a recoverable copy. A rollback after new
writes requires another pause and a synchronized copy back, so the unchanged state database and
workspace files still describe the same resources. Changing the mount does not retrofit quotas
onto old resources whose recorded storage limit is absent.

When the chart enables the sibling Identity, Connectors, Workspace, and Agent Platform components,
it supplies their private service origins to Agent Platform through explicit `AGENT_PLATFORM_*`
inputs. Enabling Agent Platform persistence also supplies its state path inside the mounted volume,
so task journals and suspended approval checkpoints are restart-safe. Workspace continues to own
the base and writable materialization references; neither Devcenter nor Agent Platform receives a
second file store.

For an isolated renderer/transport test, Workspace's loopback `workspace-terminal-lab` can replace
only the review terminal emulator:

```bash
DEVCENTER_REVIEW_TERMINAL_UPSTREAM=ws://127.0.0.1:8095 pnpm --dir frontend review
```

The terminal profile and process row are then labelled `real daemon lab`, and terminal WebSockets
are bridged only to a loopback `ws:` or `wss:` origin. Project files, agents, grants, and other
review data remain explicit samples. With the variable absent, review mode continues to use its
non-executing protocol emulator.

`DEV_CENTER_IDENTITY_PROVIDERS` contains only opaque Identity-owned IDs and display labels. With
zero entries, Identity keeps its existing selection behavior; with one, `/auth/sso/start` remains a
single-click flow; with several, Devcenter requires an explicit choice. It never uses email for
account linking.

The MCP publication store supports SQLite and PostgreSQL and owns only credential-free publication,
immutable revision, client metadata, pending approval, and audit-reference records. The chart
requires hosted values to inject the PostgreSQL URL from `devcenter.database.existingSecret`; it is
never rendered into a ConfigMap. A publication URL is `/mcp/{opaque_id}` and its publication-specific
RFC 9728 discovery document is `/.well-known/oauth-protected-resource/mcp/{opaque_id}`. All
publications share the deployment's exact `/mcp` audience while the opaque path and immutable
projection retain publication isolation. Revoked IDs are terminal and are never reused. Identity
0.5.6 validates the short-lived human bearer and `mcp.tools.call` scope, then confidentially narrows
it into the minimum current Connector scope for each describe, approval, and invoke step. Devcenter
resolves only the immutable profile projection, while Connectors re-checks the current description,
Connection, Grant, and approval evidence. Effect-bearing calls create an exact-input approval in
the publication workspace; the owner approves or denies it there and the approval is consumed once
on an identical client retry. Agent task approvals remain distinct from MCP publication
authorization.

## Deployment CLI

`devcenterctl` verifies and renders a pinned OCI chart, checks a cluster, performs atomic Helm
upgrades, verifies the result, and rolls back to an explicit revision. It is distributed only as a
multi-arch Linux container, including `linux/arm64` for Docker on Apple Silicon.

The public chart is released as `oci://ghcr.io/beyond10x/charts/devcenter`. It can be inspected and
rendered during an evaluation, but an operational installation also needs the private application
images and deployment-specific values. The public chart alone is not a public production
distribution.

The release also publishes the `connectors-<version>` image in the private
`ghcr.io/beyond10x/devcenter` package. That component composes the generated Todo and AgentIDE
Connector factories and the SDK-owned external service-catalog factory with the ordinary hosted
Connectors runtime. A deployment chooses the exact operation exposure,
risk, approval posture, and Grants in a strict value-free overlay; the component chooses Eventlog's
PostgreSQL adapter through a Secret-backed database URL. Domain commands, validation, projections,
read-your-writes behavior, and Connector dispatch remain generated or SDK-owned rather than being
reimplemented in Devcenter.

Secrets is an inner service of that composition, not a standalone chart dependency. The chart
accepts a pre-created versioned keyring Secret and a pre-created database Secret. For development it
can run one persistent PostgreSQL StatefulSet; production values disable that StatefulSet and point
the same database-URL key at an externally managed PostgreSQL service. Connectors receives a
projected token for the exact Secrets workload audience, while Secrets receives TokenReview
authority and an exact service-account-to-tenant grant. Secret values and key bytes never appear in
chart values or rendered ConfigMaps.

<!-- b10x-docs:start -->
## Documentation

[Devcenter documentation](https://beyond10x.github.io/docs/devcenter/) · [Start](https://beyond10x.github.io/) · [Ecosystem](https://beyond10x.github.io/ecosystem/) · [Impact](https://beyond10x.github.io/changes/) · [Releases](https://beyond10x.github.io/releases/)
<!-- b10x-docs:end -->
