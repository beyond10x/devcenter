---
format: aep.planning-md/1
id: story:refresh-user-bound-model-credential
kind: story
status: draft
title: Refresh stale user-bound model credentials
summary: Do not report a model connection as ready when its stored OAuth credential can no longer be refreshed or redeemed.
relations:
- derived_from: epic:claude-subscription-connection
revision: 1
---
## Context

In the dev cluster, the Claude connection appeared connected while every new agent task failed with “Connect a user-bound model in Connections, then retry with a new task.” Disconnecting and reconnecting the same account restored execution.

The connection projection currently treats the presence of a stored secret as ready. It does not prove that the credential can still be refreshed or redeemed, so stale OAuth state remains green until task startup fails.

## Acceptance

When a stored user-bound Claude credential cannot be refreshed or redeemed, Devcenter either refreshes it before task startup or marks the connection as needing attention with an actionable reconnect state, and never continues to report it as ready.

## Evidence

- Observed in namespace `devcenter` on 2026-09-02: disconnecting and reconnecting was sufficient to restore task execution.
- `connectors/crates/subscription-custody/src/lib.rs` derives the connected projection from stored-secret presence; credential redemption is exercised later by task execution.

## Scope

- Connection health/readiness projection for user-bound model credentials.
- Credential refresh or redemption before a new task is admitted.
- UI state and recovery guidance when refresh fails.
