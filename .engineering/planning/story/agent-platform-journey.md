---
format: aep.planning-md/1
id: story:agent-platform-journey
kind: story
status: implemented
title: Manage agents and Tasks through the BFF
summary: Expose one authenticated allowlisted Agent Platform journey through Devcenter.
relations:
- derived_from: epic:authenticated-control-plane
revision: 4
---
## Outcome

An authenticated engineer can list and create an agent and submit and inspect one Task through the
Devcenter BFF.

## Acceptance

- Devcenter proxies an explicit allowlist rather than arbitrary upstream paths.
- Agent Platform resolves Identity directly and derives tenant, actor and permissions from verified
  authority and deployment policy.
- Agent creation, listing, Task submission and Task inspection work end to end.
- Requests for another tenant and permissions not granted by policy are refused by name.

