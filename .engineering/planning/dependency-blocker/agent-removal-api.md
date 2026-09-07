---
format: aep.planning-md/1
id: dependency-blocker:agent-removal-api
kind: dependency-blocker
status: cleared
title: Agent Platform has no agent removal operation
relations:
- blocks: story:agent-management-controls
withholds: test_result
revision: 3
---
# Agent removal API dependency

Agent Platform published the retirement contract and client method in the lifecycle candidate at a27f90f61d6d043b73ff6f41384cc38d90f6026b, with escaped client paths corrected at b14cb0957f1e824a3de6425078692137b02fd01a. Devcenter consumes the latter exact client revision and exposes authenticated removal through its allowlisted BFF.

The API keeps immutable task and revision evidence, enforces owner and tenant boundaries, and refuses removal during active work. Actual local browser acceptance has passed repeated creation, persisted edit and deletion through the composed service APIs. The upstream API dependency is resolved. Remaining real-model conversation acceptance belongs to the owning story and does not indicate a missing removal operation.
