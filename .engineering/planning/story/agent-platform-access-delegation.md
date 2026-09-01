---
format: aep.planning-md/1
id: story:agent-platform-access-delegation
kind: story
status: archived
title: Delegate Devcenter sessions to Agent Platform
summary: Exchange the browser session for least-privilege Agent Platform access on each allowlisted BFF operation.
relations:
- derived_from: epic:authenticated-control-plane
revision: 3
---
## Outcome

The Devcenter BFF calls Agent Platform with short-lived audience-bound access credentials rather than forwarding its browser session.

## Acceptance

- Each allowlisted BFF operation requests only its required Agent Platform scope set.
- The browser session never leaves the BFF except at Identity's token exchange boundary.
- Agent Platform refusals remain generic to the browser and credential values never enter diagnostics.
- Agent listing, creation, task submission, task inspection, and event streaming work in the dev-cluster journey.

## Out of Scope

Arbitrary upstream proxying and service-bound trigger credentials.

## Decision

Archived before implementation. Devcenter continues to pass the generic Identity session to Agent Platform's verifier; Agent Platform owns the interpretation of authority at its boundary. No Agent Platform audience or scope vocabulary is compiled into Identity, and adding or changing Agent Platform permissions must not require an Identity release.
