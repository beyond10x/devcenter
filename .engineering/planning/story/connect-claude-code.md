---
format: aep.planning-md/1
id: story:connect-claude-code
kind: story
status: implemented
title: Connect Claude Code for governed attempts
summary: Authorize a user-owned Claude subscription through Connector-owned PKCE and lease it only to an exact Harness attempt.
relations:
- derived_from: epic:claude-subscription-connection
revision: 7
---
## Outcome

An authenticated engineer authorizes the provider's public client through a Connector-owned OAuth2 PKCE flow. Connectors stores and refreshes the resulting user-bound credentials, while an exact Harness attempt can use a bounded lease without any general credential export surface.

## Acceptance

- Connectors implements S-077 for bounded, expiring, single-use PKCE start and completion.
- Connectors owns the verifier, provider exchange, refresh-token rotation, storage, replacement, and revocation.
- Devcenter keeps only an opaque flow id in browser memory while authorization is pending and clears the one-time code immediately after submission.
- Harness fetches a short-lived attempt lease at call time, drops the bearer after each call, and cannot list or export the underlying credential.
- Identity supplies generic verified authority only; it knows no Claude, connector, Agent Platform, or downstream authorization vocabulary.
- Devcenter displays connection presence and lifecycle state only.
