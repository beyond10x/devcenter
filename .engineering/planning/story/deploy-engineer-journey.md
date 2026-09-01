---
format: aep.planning-md/1
id: story:deploy-engineer-journey
kind: story
status: active
title: Deploy and verify the engineer journey
summary: Run all released components in the devcenter namespace and verify the complete internal experience.
relations:
- derived_from: initiative:engineer-journey
revision: 3
---
## Outcome

Engineers can complete the authenticated agent and Claude connection journey against the internal
Devcenter deployment.

## Acceptance

- Released, digest-pinned Identity, Agent Platform, Connectors and Devcenter workloads run in the
  `devcenter` namespace.
- CI validates artifacts, deploys through KAS and verifies internal DNS, TLS, login discovery,
  session authority, agent management, Task admission and credential lifecycle.
- Deployment-specific identifiers and secrets exist only in the private deployment repository and
  secret stores.
- Atlas records exact releases, digests and end-to-end evidence.

