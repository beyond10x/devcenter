---
format: aep.planning-md/1
id: dependency-blocker:workflow-client-contract
kind: dependency-blocker
status: open
title: Workflow lacks a supported DevCenter client contract
summary: Workflow 0.1.0 publishes domain types but no supported audience, service client, operations, or scopes for DevCenter composition.
relations:
- blocks: story:workflow-aep-composition
withholds: test_result
revision: 1
---
## Missing dependency

Workflow 0.1.0 at commit `117f9748b89e16335462822ccb0e016c345e8861` releases the
`workflow-domain` crate only. It does not publish the audience, official service client, stable
list or inspect operations, or scopes required by `story:workflow-aep-composition`.

## Resolution

The Workflow owner releases the supported service boundary and immutable client artifact. DevCenter
can then pin that release and implement only the allowlisted BFF operations described by the story.

## Constraint

DevCenter must not import Workflow domain internals or invent deployment coordinates as a substitute
for the missing service contract.
