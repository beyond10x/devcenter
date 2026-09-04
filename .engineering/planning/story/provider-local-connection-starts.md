---
format: aep.planning-md/1
id: story:provider-local-connection-starts
kind: story
status: implemented
title: Keep provider connection starts independent
summary: Concurrent curated provider starts must not share or clear one global in-flight state.
relations:
- derived_from: story:refresh-gitlab-authority
scope:
- confidence: cited
  path: frontend/src/features/connections
- confidence: cited
  path: frontend/tests/connections.test.ts
revision: 5
---
## Context

Curated provider authorization uses one global in-flight provider string. If two providers start concurrently, the first completion clears the global flag and re-enables the other provider while its own request is still running.

## Acceptance

Each curated provider owns its own in-flight start state. Starting, completing, or failing one provider never changes another provider's disabled or pending state, with a concurrent GitLab and Slack test proving the behavior.
