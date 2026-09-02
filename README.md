# Devcenter

Devcenter is a generic control surface for governed agents, workflows, connectors, and sandboxed
execution. This public repository contains the application source and deployment CLI, plus the
source of the public, configuration-neutral Helm chart. The repository remains proprietary under
the included license; application and deployment CLI container images remain private. Devcenter
does not publish native binary archives.

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
Platform.

### Run the full service container

Build the same Linux container used by deployments. The token lets Cargo fetch the GitHub-hosted
Rust dependencies through HTTPS and is not retained in an image layer:

```bash
DEV_CENTER_BUILD_TOKEN="$(gh auth token)"
docker build \
  --secret id=github_token,env=DEV_CENTER_BUILD_TOKEN \
  --target server \
  --tag devcenter:local \
  .
```

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
DEV_CENTER_DATABASE_URL=postgresql://... \
DEV_CENTER_AGENT_PLATFORM_ORIGIN=https://agents.example.test \
DEV_CENTER_CONNECTORS_API_BASE=https://connectors.example.test/api/connectors/v1
```

The browser receives only an opaque, Secure, HttpOnly session cookie. `Connect Claude` starts a
Connector-owned OAuth2 PKCE flow: Devcenter retains only an opaque flow id in browser memory while
the user authorizes, and Connectors owns provider exchange, refresh, and credential custody. Agent
Platform receives an attempt-bound lease and Harness redeems it only at the provider request
boundary. Agent Connector calls use separate attempt-bound invoke authority. If a call requires a
human decision, the BFF obtains short-lived approval authority, asks Connectors to seal the exact
operation, connection, description lease, and input, and hands the one-use proof directly to Agent
Platform. Neither credential reaches the browser. Identity remains provider- and service-agnostic
throughout.

`DEV_CENTER_IDENTITY_PROVIDERS` contains only opaque Identity-owned IDs and display labels. With
zero entries, Identity keeps its existing selection behavior; with one, `/auth/sso/start` remains a
single-click flow; with several, Devcenter requires an explicit choice. It never uses email for
account linking.

The MCP publication store supports SQLite and PostgreSQL and owns only credential-free publication,
immutable revision, client metadata, pending approval, and audit-reference records. The chart
requires hosted values to inject the PostgreSQL URL from `devcenter.database.existingSecret`; it is
never rendered into a ConfigMap. A publication URL is `/mcp/{opaque_id}` and its exact RFC 9728 document is
`/.well-known/oauth-protected-resource/mcp/{opaque_id}`. Revoked IDs are terminal and are never
reused. The production MCP bearer and invocation path stays fail-closed until released Identity
clients expose exact-resource OAuth claims and workload exchange. Agent task approvals use the
released Agent Platform and Connectors seams and remain distinct from MCP publication
authorization.

## Deployment CLI

`devcenterctl` verifies and renders a pinned OCI chart, checks a cluster, performs atomic Helm
upgrades, verifies the result, and rolls back to an explicit revision. It is distributed only as a
multi-arch Linux container, including `linux/arm64` for Docker on Apple Silicon.

The public chart is released as `oci://ghcr.io/beyond10x/charts/devcenter`.

The release also publishes `ghcr.io/beyond10x/devcenter-connectors`. That component composes the
generated Todo Connector factory with the ordinary hosted Connectors runtime. A deployment chooses
the exact operation exposure, risk, approval posture, and Grants in a strict value-free overlay;
the component chooses Eventlog's PostgreSQL adapter through a Secret-backed database URL. Domain
commands, validation, projections, read-your-writes behavior, and Connector dispatch remain
generated or SDK-owned rather than being reimplemented in Devcenter.

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
