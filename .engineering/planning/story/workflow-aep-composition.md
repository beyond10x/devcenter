---
format: aep.planning-md/1
id: story:workflow-aep-composition
kind: story
status: proposed
title: Compose Workflow and AEP services
summary: Add released Workflow and AEP seams to the governed engineer workspace.
relations:
- derived_from: initiative:engineer-journey
revision: 2
---
## Outcome

An engineer composes an agent with a released Workflow and AEP context through the same governed
Vue surface used for connectors and task execution.

## Context

Workflow and AEP Service are intentional independent service boundaries and are not yet composed in
Devcenter. Integration starts only after each owner releases an audience, client, and stable product
contract; Devcenter remains an allowlisted BFF and does not duplicate their domain semantics.

## Acceptance

- Workflow and AEP Service audiences and official clients are released and pinned before Devcenter
  enables either component.
- Identity registers the audiences as opaque deployment data; Devcenter exchanges only the exact
  scopes needed for the selected operation.
- The BFF exposes explicit list, inspect, and composition routes rather than arbitrary upstream
  proxying, with tenant and actor derived exclusively from the session.
- Vue lets an engineer select a workflow and relevant AEP artifact when configuring an agent, shows
  unavailable/refused states by name, and links the resulting references into the task record.
- Contract, cross-service refusal, browser, chart-disabled, and deployment tests pass without
  embedding planning records or deployment configuration in product assets.

## Out of Scope

Implementing Workflow or AEP semantics, accepting arbitrary service URLs, or storing their bearer
credentials.

## Open Questions

Which minimal released read/write operations and scopes form the first useful composition slice?
