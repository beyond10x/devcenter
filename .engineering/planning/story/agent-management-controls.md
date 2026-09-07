---
format: aep.planning-md/1
id: story:agent-management-controls
kind: story
status: active
title: Manage agents, profiles and separate conversations
summary: Edit and remove agents and capability profiles; manage durable isolated conversations through the composed APIs.
relations:
- derived_from: epic:authenticated-control-plane
- informed_by: story:agent-platform-journey
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-http/src/agent_lifecycle.rs
- confidence: cited
  path: crates/devcenter-http/src/lib.rs
- confidence: cited
  path: frontend/acceptance
- confidence: cited
  path: frontend/acceptance/lifecycle.spec.ts
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/api/schema.gen.ts
- confidence: cited
  path: frontend/src/components/ConfirmDialog.vue
- confidence: cited
  path: frontend/src/features/agents
- confidence: cited
  path: frontend/src/features/profiles
- confidence: cited
  path: frontend/src/stores/workspace.ts
- confidence: cited
  path: frontend/src/styles/main.css
- confidence: cited
  path: frontend/tests/workspace.test.ts
- confidence: cited
  path: openapi.json
revision: 16
---
# Manage agents, profiles and separate conversations

## Outcome

An authenticated engineer can create agents repeatedly, persist edits and deliberately remove agents and capability profiles. Each agent exposes separate durable conversations with selection, rename, clear and delete controls.

## Acceptance

- Agent creation can be repeated, edits survive reload, and confirmed removal selects another agent or presents the empty state. Refusals preserve the current selection.
- Default capability profile creation passes the upstream schema. Profiles can be edited and removed, with an actionable refusal while assigned to an agent.
- Conversations can be created, selected, renamed, cleared and deleted. Real model replies prove history recall within the selected conversation and isolation after creation or clearing.
- Lifecycle requests use explicit authenticated BFF routes and server-derived owner and tenant. Immutable tasks and revisions remain retained; active work prevents destructive lifecycle changes.
- Client, BFF, store and browser regression checks cover success, refusal, stale-response and concurrent-update behavior. Existing project chat, coding chat, Files and terminal acceptance remain required.

## Implementation and verification

The lifecycle API dependency is resolved by the published Agent Platform candidate, consumed through its exact official client revision. Local service images contain the lifecycle operations and matching Devcenter UI. Actual retained-deployment checks passed repeated agent creation, persisted edit and removal, profile creation and bulk posture updates, in-use deletion refusal and successful removal after release. Frontend checks passed 55 unit tests; browser regressions passed 38 with 18 existing conditional skips. Full required source gates passed.

Conversation UI and durable backend operations are implemented with server-derived history. One full live conversation run passed, but later composed runs failed when the provider refused the recall request. That later failure supersedes the earlier pass for readiness. The UI displays failure alongside partial output; no refused task is counted as successful.

The bounded provider diagnostic reports refusal category reasoning_extraction. The Agent Platform candidate at 2d391c83e137b51b723042d5cf1bf8c830b7b2ca now sends prior messages as typed user/assistant history rather than flattening them into a user prompt. Its full source checks passed, but the unchanged real conversation test still received the same refusal after local deployment. The provider explanation is not proof that this benign test violates policy. Live model reliability remains unresolved; no release or shared-environment promotion is ready.

## Evidence retention

Private evidence records exact images, task identifiers and outcomes. One diagnostic helper accidentally reused a prior evidence directory and overwrote its raw test log; the newer evidence includes an explicit disclosure. The earlier failed task identifier remains recorded. Both diagnostic helpers now create unique directories and refuse reuse. Do not claim that every historical raw log survived.
