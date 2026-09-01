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

The browser receives only an opaque, Secure, HttpOnly session cookie. `Connect Claude` starts a
Connector-owned OAuth2 PKCE flow: Devcenter retains only an opaque flow id in browser memory while
the user authorizes, and Connectors owns provider exchange, refresh, and credential custody. Agent
Platform receives an attempt-bound lease and Harness redeems it only at the provider request
boundary. Identity remains provider- and service-agnostic throughout.

## Deployment CLI

`devcenterctl` verifies and renders a pinned OCI chart, checks a cluster, performs atomic Helm
upgrades, verifies the result, and rolls back to an explicit revision. It takes all environment
coordinates as arguments; none are compiled into the binary.

```console
cargo run --bin devcenterctl -- --help
```

The public chart is released as `oci://ghcr.io/beyond10x/charts/devcenter`.

## Optional hosted credential store

The chart can package one internal Vault for Connectors. It is disabled by default. An enabled
deployment must supply immutable Vault and backup images, retained storage classes, internal TLS,
the Kubernetes API CIDRs required for TokenReview, and an auto-unseal workload identity. Provider
credentials remain owned by Connectors; the chart mounts only the Vault CA into that workload.

`devcenterctl infrastructure aws ensure-vault` creates or verifies the non-secret AWS coordinates
for the supported EKS profile: a dedicated rotating KMS key, exact-service-account IAM roles, and a
private encrypted versioned backup bucket. It prints JSON coordinates suitable for a downstream
values file and never writes deployment values into this repository.

The first install uses `devcenterctl apply --initialize-vault` from an operator session with an
available OS Secret Service. Initialization stores one recovery share in that keyring, configures
restricted Kubernetes roles, and revokes rather than persists the initial root token. Ordinary pod
restarts use KMS and have no workstation dependency.

The keyring item is recovery material, not a long-lived Vault token. It is needed only for recovery
operations such as generating a temporary root token. Back it up through the operator keyring's
normal protected backup mechanism and keep access to the AWS KMS key under infrastructure change
control. Losing both retained Raft data/backups and their KMS key is unrecoverable.

The backup CronJob streams a Vault-native Raft snapshot to an opaque timestamped object under the
configured prefix. A snapshot contains Vault's encrypted internal state and is still treated as
sensitive. It does not contain the KMS key, recovery material, Connectors SQLite state, TLS Secret,
or audit-log file.

Before retiring a predecessor store, operators run `vault backup`, install a fresh one-replica
Vault in an isolated drill namespace with the same KMS workload identity, and run
`vault restore-drill`. The drill refuses an initialized target, force-restores the newest snapshot,
waits for KMS auto-unseal, and verifies the expected mount through a short-lived Kubernetes token.
`vault migrate-kv` then copies only the selected tenant subtree, preserves mutable KV metadata,
refuses conflicting destination records, verifies every value after write, and deletes its
temporary migration policy and role. `vault verify` finally proves exact tenant access, denial of a
different tenant, read/write behavior, and destructive cleanup.
