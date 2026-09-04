---
format: aep.planning-md/1
id: story:real-agentide-workspace
kind: story
status: active
title: Compose the real AgentIDE workspace
summary: Replace Devcenter's local workbench implementation with the released AgentIDE controller over one Git-backed Workspace session.
relations:
- decomposes: initiative:engineer-journey
- depends_on: story:ess-release-model
- depends_on: story:reconcile-independent-stack-units
scope:
- confidence: cited
  path: Cargo.toml
- confidence: inferred
  path: crates/devcenter-http
- confidence: inferred
  path: deploy/charts/devcenter
- confidence: cited
  path: ess/system
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: frontend/src/features/workbench
revision: 5
---
# Story: Compose the real AgentIDE workspace

## Outcome

An authenticated engineer opens any admitted GitLab repository and uses the released AgentIDE workbench over one exact Git-backed Workspace session without a product-local editor state machine.

## Acceptance

- The project shell acknowledges and displays durable preparation without blocking project navigation.
- Devcenter implements only the authenticated AgentIDE host port and selects the Vue target.
- Existing project, file, diff, terminal, coordination, approval, and agent-turn authority stays server-derived and actor-private.
- The old Devcenter workbench controller, Monaco/Ghostty ownership, and duplicate AgentIDE contract graph are removed.
- The generic chart configures the internal Git byte plane, TLS, and NetworkPolicy without deployment-specific values.
- A deployed canary proves open, edit, save, diff, terminal, streamed Markdown chat, reload/resume, and cross-user refusal before atomic promotion.

## Scope

- `ess/system` — cited
- `frontend/src/features/workbench` — cited
- `crates/devcenter-http` — inferred
- `deploy/charts/devcenter` — inferred
- `Cargo.toml` — cited
- `frontend/package.json` — cited
