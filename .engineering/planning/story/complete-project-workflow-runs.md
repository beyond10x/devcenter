---
format: aep.planning-md/1
id: story:complete-project-workflow-runs
kind: story
status: active
title: Complete project Workflow runs
summary: Turn an accepted project workflow into observable execution and one durable terminal result.
relations:
- derived_from: initiative:engineer-journey
- informed_by: story:durable-task-recovery
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src/features/projects
- confidence: cited
  path: frontend/src/stores/workspace.ts
revision: 4
---
## Context

Project-bound workflows are listed by Workspace and their start endpoint accepts work, but the current user experience can remain at “accepted” with no progress or result. Admission is not useful functionality unless execution ownership and recovery produce an observable terminal state.

## Acceptance

Starting any pre-built workflow from an admitted repository moves visibly through accepted and running to exactly one succeeded or named failed terminal result with streamed output, and a worker or browser restart resumes observation without replaying effects or leaving the run indefinitely accepted.

## Scope

The released Workspace and Agent Platform execution contracts, Devcenter component pins and allowlisted BFF routes, the project Workflow progress UI, and disruption-based authenticated qualification.
