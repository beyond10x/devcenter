---
format: aep.planning-md/1
id: story:agentide-renderer-consumption
kind: story
status: active
title: Consume the AgentIDE Vue renderer
summary: Keep hosted transport in Devcenter while rendering through the released AgentIDE Vue target.
relations:
- decomposes: epic:authenticated-control-plane
- derived_from: initiative:engineer-journey
scope:
- confidence: inferred
  path: CHANGELOG.md
- confidence: inferred
  path: Cargo.toml
- confidence: inferred
  path: crates/devcenter-connectors
- confidence: inferred
  path: deploy/charts/devcenter
- confidence: inferred
  path: frontend
- confidence: inferred
  path: openapi.json
revision: 6
---
## Outcome

Devcenter's hosted coding workspace uses the exact released AgentIDE Vue renderer composition
shell. Identity, Workspace, Agent Platform, terminal transport, routing, persistence, and authority
remain in Devcenter-owned host adapters.

## Acceptance criteria

- The frontend pins the exact released AgentIDE renderer source commit and imports its public Vue
  target.
- AgentIDE owns the workbench layout and accessibility landmarks; Devcenter provides host-owned
  editor, diff, agent, inspector, terminal, and overlay projections through named regions.
- Existing files, saves, diffs, terminals, context attachments, approvals, and agent turns remain
  available.
- The renderer source contains no Devcenter endpoint, bearer, deployment, or organization-specific
  identifier.

## Evidence required

- Complete frontend and repository gates.
- Authenticated hosted-workbench smoke test and successful agent turn after deployment.
