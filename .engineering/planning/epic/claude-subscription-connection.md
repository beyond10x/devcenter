---
format: aep.planning-md/1
id: epic:claude-subscription-connection
kind: epic
status: active
title: Connect a user-owned Claude Code subscription
summary: Connectors owns subscription custody and Harness consumes an attempt-bounded lease.
relations:
- derived_from: initiative:engineer-journey
revision: 3
---
## Outcome

An authenticated engineer can connect, inspect and revoke their own Claude Code subscription for
agent work without exposing credential bytes to Devcenter or persistence outside Connectors.

## Acceptance

- The catalog declares the custody-only Claude Code provider.
- The hosted Connectors posture accepts a self-service Connect Session for that provider.
- A reviewed lease contract lets an authorized Harness attempt obtain the credential without a
  general export/read API.
- The UI reports lifecycle metadata without returning the credential.

