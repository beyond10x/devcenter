---
format: aep.planning-md/1
id: dependency-blocker:workflow-client-contract
kind: dependency-blocker
status: cleared
title: Workflow lacks a supported DevCenter client contract
summary: Workflow 0.1.0 publishes domain types but no supported audience, service client, operations, or scopes for DevCenter composition.
relations:
- blocks: story:workflow-aep-composition
withholds: test_result
revision: 3
---
## Missing dependency

Workflow 0.1.0 at commit `117f9748b89e16335462822ccb0e016c345e8861` released only the `workflow-domain` crate. It did not publish the audience, official service client, stable list or inspect operations, or scopes required for DevCenter composition.

## Resolution

Workflow 0.2.0 at commit `ccff54bcb65555482d8f74639e43e91ec970c3e6` publishes its Service SDK-generated client, `urn:b10x:workflow` audience, `workflows.read` and `workflows.manage` scopes, and the supported definition operations. DevCenter 0.8.12 consumes only the official read operations and exact read authority.

## Constraint

Satisfied: DevCenter imports the generated Workflow service client and does not import Workflow domain internals or invent deployment coordinates.
