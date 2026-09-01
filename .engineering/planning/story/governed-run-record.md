---
format: aep.planning-md/1
id: story:governed-run-record
kind: story
status: proposed
title: Read the governed run record
summary: Explain attempts, authority, evidence, and refusals in the Vue workspace.
relations:
- derived_from: initiative:engineer-journey
revision: 2
---
## Outcome

An engineer can read a coherent governed run record that explains an attempt, the authority it used,
the operations it invoked, its evidence, and any named refusal without inspecting raw service logs.

## Context

The deployed workspace can submit work and stream output, but O1 and O5 require the product to make
the governed execution record understandable. The record remains owned by the services that execute
and authorize the work; Devcenter composes released read models only.

## Acceptance

- Devcenter pins the released Agent Platform and supporting client contracts that expose attempt,
  capability-lease, operation, evidence, and refusal read models.
- The BFF exposes only exact read-only routes and derives tenant and actor from the Identity session.
- Vue presents an ordered attempt timeline with running, completed, failed, canceled, and refused
  states, and distinguishes a named policy refusal from an infrastructure failure.
- Capability and connector metadata remain credential-free; no token, lease bearer, or credential
  bytes enter browser state, logs, documentation, or Devcenter persistence.
- Contract, refusal-vector, accessibility, and browser tests prove the record against released
  service seams.

## Out of Scope

Owning the attempt ledger, reinterpreting arbitrary logs, minting authority, or storing credentials.

## Open Questions

Which release first publishes the stable governed-run read model and pagination contract?
