# Devcenter

Devcenter is a generic control surface for governed agents, workflows, connectors, and sandboxed
execution. This repository contains the private application source and deployment CLI, plus the
source of the public, configuration-neutral Helm chart.

The repository deliberately contains no real deployment values. A deployment supplies its tenant,
hosts, image mirrors, Identity configuration, connector catalogue bundle, storage classes, and
Secret references separately.

## Run locally

```console
DEV_CENTER_TENANT_ID=local \
DEV_CENTER_PUBLIC_ORIGIN=http://127.0.0.1:8080 \
cargo run --bin devcenter
```

Open `http://127.0.0.1:8080/docs/` for the embedded documentation and
`http://127.0.0.1:8080/openapi.json` for the service contract.

Protected routes fail closed until an Identity verifier is configured. For loopback-only MCP
development, also set `DEV_CENTER_INSECURE_DEV_AUTH=true` and a non-empty
`DEV_CENTER_DEV_BEARER_TOKEN`.

In a production posture, set the Identity origin and exact web callback, plus the private inner
service origins:

```console
DEV_CENTER_IDENTITY_ORIGIN=https://identity.example.test \
DEV_CENTER_IDENTITY_AUDIENCE=urn:b10x:devcenter \
DEV_CENTER_IDENTITY_WEB_CLIENT_ID=devcenter-web \
DEV_CENTER_IDENTITY_REDIRECT_URI=https://devcenter.example.test/auth/sso/callback \
DEV_CENTER_AGENT_PLATFORM_ORIGIN=https://agents.example.test \
DEV_CENTER_CONNECTORS_API_BASE=https://connectors.example.test/api/connectors/v1 \
cargo run --locked -p devcenter-app
```

The browser receives only an opaque, Secure, HttpOnly session cookie. Subscription credentials are
forwarded once into Connectors custody; Agent Platform receives an attempt-bound lease and Harness
redeems it only at the provider request boundary.

## Deployment CLI

`devcenterctl` verifies and renders a pinned OCI chart, checks a cluster, performs atomic Helm
upgrades, verifies the result, and rolls back to an explicit revision. It takes all environment
coordinates as arguments; none are compiled into the binary.

```console
cargo run --bin devcenterctl -- --help
```

The public chart is released as `oci://ghcr.io/beyond10x/charts/devcenter`.
