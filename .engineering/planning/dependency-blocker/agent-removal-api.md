---
format: aep.planning-md/1
id: dependency-blocker:agent-removal-api
kind: dependency-blocker
status: open
title: Agent Platform has no agent removal operation
relations:
- blocks: story:agent-management-controls
withholds: test_result
revision: 1
---
# Dependency blocker: Agent Platform has no agent removal operation

## Missing dependency

DevCenter pins Agent Platform commit 3df4e57218232ab28c1f9390c4fd1f3c94d66e91. That contract exposes list, create, get, revision, and activation operations for agents, but no delete, deactivate, or archive operation. The current Agent Platform remote main commit fa5b697f0931f26cd8dd3968a62e02927307bab9 has the same gap.

## Consequence

DevCenter cannot truthfully remove an agent through its allowlisted BFF. A browser-only tombstone would return after refresh and would not change platform authority, so it is explicitly outside this story.

## Clears when

The Agent Platform repository publishes a governed retirement operation and client method with defined behavior for retained tasks, active attempts, triggers, and immutable revisions. A coordinator can then pin that exact revision and complete the DevCenter BFF and UI removal flow.
