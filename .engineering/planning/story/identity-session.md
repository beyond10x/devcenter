---
format: aep.planning-md/1
id: story:identity-session
kind: story
status: implemented
title: Sign in through Identity
summary: Complete generic SSO and derive the Devcenter session from Identity authority.
relations:
- derived_from: epic:authenticated-control-plane
revision: 4
---
## Outcome

An engineer can sign in to Devcenter through generic Identity-backed SSO and receive a session
whose tenant and actor are resolved only by Identity.

## Acceptance

- Identity ships its configured, versioned audience registry before adding Devcenter.
- Devcenter uses authorization-code plus S256 PKCE with exact callback and state validation.
- The Identity session is stored only in a Secure, HttpOnly, SameSite cookie and is never available
  to application JavaScript or logs.
- `/api/session` returns non-secret verified subject, tenant, email and groups.
- Wrong audience, expired, revoked, malformed and missing credentials fail closed in shared tests.

