---
format: aep.planning-md/1
id: story:connect-claude-code
kind: story
status: implemented
title: Connect Claude Code for governed attempts
summary: Store a user-owned setup token in Connectors and lease it only to an authorized Harness attempt.
relations:
- derived_from: epic:claude-subscription-connection
revision: 4
---
## Outcome

An authenticated engineer connects a Claude Code setup token to their own Connector identity and a
Harness attempt can use it without any credential export surface.

## Acceptance

- Connectors implements S-071 and S-073 for the custody-only provider and hosted self-service flow.
- The flow instructs the person to obtain the token with vendor tooling; Devcenter never drives or
  impersonates the vendor OAuth client.
- Credential storage, replacement and revocation are owner-scoped and transactionally durable.
- Harness fetches a short-lived attempt lease at call time, drops the bearer after each call and
  cannot list or export the underlying credential.
- Devcenter displays connection presence and lifecycle state only.

