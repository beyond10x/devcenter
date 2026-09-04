---
format: aep.planning-md/1
id: story:refresh-user-bound-model-credential
kind: story
status: active
title: Refresh stale user-bound model credentials
summary: Do not report a model connection as ready when its stored OAuth credential can no longer be refreshed or redeemed.
relations:
- derived_from: epic:claude-subscription-connection
scope:
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src/features/agents
- confidence: cited
  path: frontend/src/features/connections
revision: 5
---
## Outcome

A user-bound model connection is reported ready only when it can fund a new attempt, and an engineer can repair it from the same Connections surface when it cannot.

## Context

The current connection projection derives “Connected” from stored-secret presence, but redemption happens later during task startup. Stale OAuth state is therefore green in Connections and fails only after the engineer submits work.

## Acceptance

Before admitting a new task, the system refreshes or validates the user-bound model credential and either streams a terminal agent result or changes the connection to a named needs-attention state with a working reconnect/revoke action, never leaving an unusable credential labeled Connected.

## Scope

Connector model-credential readiness, task-admission credential redemption, Devcenter's model status projection and recovery UI, and an authenticated agent-turn smoke check.
