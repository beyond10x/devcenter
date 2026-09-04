---
format: aep.planning-md/1
id: story:refresh-gitlab-authority
kind: story
status: active
title: Refresh GitLab authority without rebuilding Devcenter
summary: Promote the fixed Connectors runtime as an independent Devcenter release unit and restore live repository reads.
relations:
- decomposes: epic:independent-component-delivery
scope:
- confidence: cited
  path: .github/workflows/promote-connectors.yml
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
revision: 6
---
# Story: Refresh GitLab authority without rebuilding Devcenter

## Outcome

An engineer's GitLab repository grant continues to authorize project and file reads after OAuth access-token expiry, and operators can promote that fix as only the Devcenter Connectors runtime.

## Context

GitLab may omit an unchanged `scope` field from a valid refresh response. Connectors 0.5.11 accepts that response and still verifies refreshed authority before committing rotation. The generated AgentIDE and Todo factories must share its exact factory graph through Service SDK 0.5.6.

## Acceptance

- `crates/devcenter-connectors` pins the merged Connectors 0.5.11, Service SDK 0.5.6, AgentIDE 0.3.2, and Todo 0.3.3 commits.
- The nested lock resolves one Connector factory graph and its fmt, clippy, and tests pass.
- A default-branch-only promotion publishes and signs only the Connectors multi-platform image.
- The promotion emits an immutable manifest containing the source commit and image digest, without rebuilding the Devcenter server, deployment CLI, or chart.
- After the one necessary GitLab reconnect, live project listing and repository files succeed; later token refresh retains access.

## Out of Scope

Changing GitLab repository visibility, accepting upstream tokens as Devcenter bearers, or widening a user's grant.

## Open Questions

None.
