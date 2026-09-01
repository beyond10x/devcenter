# AGENTS.md — devcenter

## Serves

- **O1 — governed reach.** People and workloads receive only authority derived from Identity and current Connector Grants.
- **O4 — products run on the foundation.** Devcenter composes the released service seams into an operable product.
- **O5 — the generic agent platform.** Engineers manage agents, tasks, workflows, and connector capabilities through one neutral surface.

## Boundary

This repository owns the generic Devcenter frontend, BFF, deployment CLI, embedded documentation,
and generic Helm chart. It owns no organization-specific deployment configuration, connector
catalogue, hostname, email domain, tenant identifier, registry coordinate, or cluster credential.

The repository source and chart are public. The application images and CLI release remain private.
Deployment-specific values and automation inputs live in a downstream private repository. Public
repository visibility does not change the proprietary license unless that license is changed
explicitly.

## Invariants

1. Identity issues every Devcenter session. An upstream provider token is never a Devcenter bearer.
2. Tenant and actor are server-derived and never accepted from request payloads.
3. The BFF exposes only explicitly allowlisted routes.
4. User-bound model credentials remain owned by Connectors. Devcenter never stores credential bytes.
5. Generated and embedded documentation contains no deployment configuration or planning records.
6. No organization-specific identifier enters source, tests, examples, commits, planning records, release metadata, chart packages, or OCI labels.
7. Anything that runs is Rust. Shell is orchestration only.
8. The chart uses immutable image references in deployment values and never embeds credentials.

## Gate

```console
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
helm lint deploy/charts/devcenter --values deploy/charts/devcenter/ci/test-values.yaml
bash ci/check-chart-rollouts.sh
cargo run --locked --bin devcenterctl -- leak-check --root . --deny-file ci/denylist.example
```

Automated commits and pushes use the organization bot. Preserve unrelated work and never commit
deployment values, credentials, rendered Secrets, or private connector bundles.
