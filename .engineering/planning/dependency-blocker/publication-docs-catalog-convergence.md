---
format: aep.planning-md/1
id: dependency-blocker:publication-docs-catalog-convergence
kind: dependency-blocker
status: open
title: Organization documentation reconciliation needs current source checkouts
relations:
- blocks: runbook:independent-publication-wave
withholds: test_result
revision: 1
---
## Observation

The required organization docs reconcile --check run from clean Atlas remote main a8fb936ddcb35c8971311610e5c63cc86d612fab, with current managed Devcenter, Workspace, Website and Docs System overrides, exits 1:

```text
refused: documentation surface aep/docs differs from its repository manifest
```

AEP's primary checkout is stale relative to its remote main. This observation establishes a local workspace/catalog reconciliation gap, not a defect in the Workspace release. The initial attempt also encountered a stale primary Docs System collector; using the current managed collector resolved that earlier schema error.

## Clears when

An organization workspace with current AEP and other source checkouts passes the same Atlas docs reconcile --check command, or the documentation authority reconciles a demonstrated catalog mismatch. Do not mutate unrelated dirty primary checkouts to make this check pass.

## Independent evidence

Atlas docs verify-portal succeeds for the refreshed Website lock: 23 locked public sources, 24 surfaces and 50 delivery records. Both publication stories have verified live releases. This blocker withholds organization-wide documentation reconciliation evidence only; it does not block use of their published immutable artifacts.
