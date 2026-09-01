---
format: aep.planning-md/1
id: epic:authenticated-control-plane
kind: epic
status: active
title: Authenticated Devcenter control plane
summary: Identity-backed Devcenter sessions compose Agent Platform through an allowlisted BFF.
relations:
- derived_from: initiative:engineer-journey
revision: 3
---
## Outcome

An authenticated engineer can use Agent Platform through Devcenter under one verified Identity
authority.

## Acceptance

- Identity admits configured Devcenter and Agent Platform audiences.
- Devcenter completes OAuth authorization-code plus PKCE and keeps the Identity session in a
  secure HTTP-only cookie.
- Devcenter proxies only the documented agent and Task operations.
- Agent Platform resolves Identity authority and scopes every operation by verified tenant.

