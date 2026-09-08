---
format: aep.planning-md/1
id: story:downward-project-coordination
kind: story
status: active
title: Coordinate project tasks above Workspace and Agent Platform
relations:
- derived_from: initiative:engineer-journey
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: crates/devcenterctl
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: docs/local-acceptance.md
revision: 5
---
## Context

Project chat and review execution currently create a dependency cycle between Workspace and Agent Platform. The product coordinates both services above their separate ownership boundaries. This record captures the implementation already in progress and its remaining local acceptance before publication.

## Acceptance

An admitted engineer can send project chat and start every review workflow through Devcenter. Each submits one owned Agent Platform task, records its association in Workspace, and observes one durable terminal result with restart recovery. Workspace verifies separate coordinator and executor request proofs with current Identity and grants. Existing conversation history, task ownership, coding sessions and tool authorization remain enforced.

The actual local Helm composition must pass real model replies in main Agents, project chat and coding chat, file save and exact restoration, real PTY execution and termination, and acknowledged workspace cleanup. Existing model integration, authentication and credential custody remain unchanged.

## Scope

The BFF owns submission and observation, Workspace retains context and conversation data, and the chart and local CLI provision distinct downward service authority. Existing typed Workspace task and request-proof contracts are the authority; the product adds no duplicate authority schema. Local key preparation must survive interrupted generation without publishing an empty key or rotating a retained valid key. Source dependencies and the lockfile must resolve the published Workspace source before publication.

## Evidence and remaining work

The Rust service tests and strict Clippy, frontend checks and browser tests, standalone Connector checks, chart checks and version/leak checks passed for the implementation candidate. Required actual local composition acceptance and final published dependency resolution remain pending. No production deployment is part of this repair's acceptance.
