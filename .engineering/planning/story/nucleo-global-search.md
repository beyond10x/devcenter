---
format: aep.planning-md/1
id: story:nucleo-global-search
kind: story
status: draft
title: Rank Devcenter global search with Nucleo
summary: Use Rust Nucleo ranking over the authenticated Search all candidate set.
relations:
- derived_from: story:global-search-hotkeys
revision: 1
---
## Context

`story:global-search-hotkeys` delivered authority-bounded Search all with exact, prefix, token, and
substring ranking. Its current scorer is handwritten in `frontend/src/app/search.ts:26`, while
candidate fan-out and partial-failure handling live in `frontend/src/app/GlobalSearch.vue:42`.
Nucleo provides mature fuzzy subsequence ranking suitable for abbreviated resource names, but the
matcher must not move authorization or expose candidates a caller cannot already read.

## Scope

- cited: `frontend/src/app/search.ts`, `frontend/src/app/GlobalSearch.vue`, `frontend/src/api/client.ts`, `crates/devcenter-http/src/lib.rs`, the generated OpenAPI contract, and global-search unit/e2e coverage.
- inferred: a bounded Devcenter-owned Rust search module using Nucleo, an authenticated BFF ranking endpoint over existing typed authority-checked clients, and frontend projection of ranked resource identities into existing navigation targets.

## Constraints

- Use Nucleo's matcher in Rust; do not introduce a second service or let the browser submit an arbitrary candidate corpus for scoring.
- Acquire candidates only through existing authenticated, tenant- and actor-derived service clients, then rank the admitted set.
- Preserve exact and prefix boosts, stable identity tie-breaking, per-group bounds, stale-request rejection, named partial failures, and navigation-only selection.
- Do not persist or share a user's candidate index across principals, and bound both candidate count and query length.

## Acceptance

An authenticated user can retrieve stable, group-bounded Search all results for exact, prefix, and non-contiguous abbreviated queries through Nucleo ranking, with every result proven readable by that user's existing authority and partial source failures still named without revealing hidden candidates.
