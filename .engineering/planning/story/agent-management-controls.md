---
format: aep.planning-md/1
id: story:agent-management-controls
kind: story
status: active
title: Remove agents deliberately
summary: Add governed agent retirement once Agent Platform defines the operation.
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
revision: 14
---
# Story: Remove agents deliberately

## Outcome

An authenticated engineer can deliberately retire an agent without losing immutable task and revision history or hiding only local browser state.

## Acceptance

- The selected agent exposes a deliberate remove action with confirmation.
- A successful retirement selects a remaining agent or shows the empty state; a refusal preserves the current selection.
- Removal is an authenticated Agent Platform operation exposed through the allowlisted DevCenter BFF.
- The operation defines behavior for retained tasks, active attempts, triggers, and immutable revisions.
- Store, BFF, client, and browser regression coverage proves confirmation, success, and refusal behavior.

## Scope

- Selected-agent removal and confirmation UI.
- Workspace removal state and route reconciliation.
- DevCenter API client, allowlisted BFF route, generated OpenAPI contract, and Agent Platform dependency once the upstream operation exists.
- Focused unit, BFF, and browser regression coverage.

## Current implementation scope

The operator requested completion of agent editing and removal, durable conversation separation and deletion, and capability-profile creation and removal. The local browser reproduced default profile creation failing with HTTP 422; two successive agent creates both returned HTTP 201. Implement the missing upstream lifecycle operations with principal ownership, retained immutable execution evidence, active-work refusal, and durable visibility. Expose them through the allowlisted BFF and browser controls, then validate against locally built service images before any release. Preserve existing Agents, project chat, coding chat, Files and terminal acceptance. Local real-provider configuration is tracked by the related local integration story.

## Local verification

Deployed agent/profile lifecycle acceptance passed twice through the real UI and APIs: repeated creation, edit and reload, deletion, default capability profile creation, bulk changes, assigned-profile refusal and deletion after release. One real Claude conversation run passed recall after reload, new-conversation isolation, clear and delete. The later final composed run failed with provider-declined stop reasons in both the conversation and existing main-agent acceptance; its result supersedes the earlier pass for release readiness. Earlier opaque-code and EMPTY diagnostics also received provider refusals. All failures are retained, never counted as successful tasks. Agent Platform reports bounded incomplete-stop reasons, and the latest deployed UI displays the failure alongside partial output. The provider's exact refusal category is not yet available through the pinned Harness adapter. Source gates and lifecycle CRUD pass; live model reliability remains unresolved and no release is authorized by this evidence.

## Local acceptance evidence

The actual retained local deployment passed repeated agent creation, persisted edit and deletion, capability-profile creation/bulk posture updates, in-use deletion refusal and later deletion. The complete browser regression suite passed 38 tests with 18 existing conditional skips; frontend unit checks passed 55 tests. Conversation UI and durable backend operations are implemented with server-derived context, but live model acceptance remains incomplete: one complete context/isolation/clear/delete run passed, then a later composed run failed with a provider refusal. The UI now displays the failure together with partial output. Keep this outcome open until live reliability is resolved and verified; an earlier pass does not supersede the later failure.
