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
7. Servers, BFFs, CLIs, deployment tooling, and orchestration are Rust. The browser application is
   Vue and TypeScript, built by the repository-pinned Node and pnpm toolchain.
8. The chart uses immutable image references in deployment values and never embeds credentials.

## Gate

```console
pnpm --dir frontend install --frozen-lockfile
pnpm --dir frontend check
pnpm --dir frontend exec playwright install chromium
pnpm --dir frontend test:e2e
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
bash ci/check-version-consistency.sh
helm lint deploy/charts/devcenter --values deploy/charts/devcenter/ci/test-values.yaml
bash ci/check-chart-rollouts.sh
cargo run --locked --bin devcenterctl -- leak-check --root . --deny-file ci/denylist.example
```

Automated commits and pushes use the organization bot. Preserve unrelated work and never commit
deployment values, credentials, rendered Secrets, or private connector bundles.

<!-- b10x-docs-operations:start -->
## Public documentation operations

This repository owns the public source and presentation allowlist in `b10x.docs.yaml`; the unified [beyond10x Website](https://beyond10x.github.io/docs/devcenter/) passively collects those declared files from the exact commit in `website/sources.lock.json`. Atlas owns discovery grouping/order; Website and Docs System own rendering, shared components, search, and feeds. Do not add a standalone docs deployer or put App credentials in this public repository. If Atlas catalogs a former Pages workflow, that file remains repository-owned validation: preserve its bespoke checks while keeping exact read-only permissions, an unconditional pull-request trigger, and no deployment primitives. Project Pages at `/devcenter/` is only the generated redirect façade in `.github/workflows/b10x-docs-pages.yml`.

From the complete organization workspace, verify the contract with a clean Atlas checkout at the current remote `main`. Set `B10X_ATLAS_CHECKOUT` to a managed Atlas worktree when the primary checkout is dirty or stale; never infer command availability from the primary alone.

```bash
atlas_checkout="${B10X_ATLAS_CHECKOUT:-atlas}"
atlas_head="$(git -C "$atlas_checkout" rev-parse HEAD)"
atlas_main="$(git -C "$atlas_checkout" ls-remote origin refs/heads/main | awk '{print $1}')"
test -z "$(git -C "$atlas_checkout" status --porcelain)"
test "$atlas_head" = "$atlas_main"
cargo run --manifest-path "$atlas_checkout/Cargo.toml" --locked -q -- \
  --store "$atlas_checkout/catalog/store" docs reconcile --workspace . --check
```

Keep internal plans, stories, ADRs, decisions, worklogs, security material, and research out of the public allowlist unless a repository authority explicitly declares them public.
<!-- b10x-docs-operations:end -->
