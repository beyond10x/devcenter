---
format: aep.planning-md/1
id: initiative:engineer-journey
kind: initiative
status: active
title: One authenticated engineer journey
summary: SSO, agent management, Tasks and user-owned Claude Code access work end to end in the dev cluster.
revision: 3
---
## Outcome

An engineer signs in to the internal Devcenter, connects a user-owned Claude Code subscription,
manages an agent, submits work and observes the result without any product service owning another
service's credentials or authority.

## Acceptance

- Generic SSO terminates in an Identity session and derives tenant and actor server-side.
- Devcenter exposes an allowlisted Agent Platform journey for agents and Tasks.
- Connectors owns Claude Code credential custody, rotation and revocation; Harness borrows only an
  attempt-bounded credential.
- The complete journey is deployed and verified in the `devcenter` namespace.

