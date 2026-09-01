---
format: aep.planning-md/1
id: story:durable-task-recovery
kind: story
status: proposed
title: Resume durable task execution
summary: Consume durable Agent Platform attempts and resumable task events without replaying effects.
relations:
- derived_from: initiative:engineer-journey
revision: 2
---
## Outcome

Task progress survives worker or browser interruption and resumes in the Vue workspace without
replaying effects or silently losing ordered events.

## Context

The current walking slice intentionally uses process-local workers and task streams. Agent Platform
must own durable execution and lease revalidation; Devcenter should consume that released seam and
make reconnection behavior visible without inventing a second task engine.

## Acceptance

- Devcenter pins the Agent Platform release that owns durable attempts and resumable event cursors.
- The BFF forwards only the exact task-status and event-stream routes and never accepts tenant or
  actor authority from browser payloads.
- Vue reconnects from the last acknowledged opaque event cursor, de-duplicates events, and renders
  explicit reconnecting, recovered, terminal, and unrecoverable states.
- Retry and recovery tests prove Agent Platform revalidates the current capability lease before any
  repeated effect and reports expired authority as a named refusal.
- A deployment disruption test proves an admitted task reaches one terminal record with ordered
  output after worker restart.

## Out of Scope

Implementing Agent Platform persistence, caching user credentials, or increasing replicas while
Identity or Devcenter session state remains process-local.

## Open Questions

What cursor and terminal-attempt contract will Agent Platform publish for reconnecting consumers?
