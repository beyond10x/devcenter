---
format: aep.planning-md/1
id: story:codex-mcp-least-privilege-setup
kind: story
status: implemented
title: Keep Codex MCP setup least-privileged
summary: Replace the generated Codex command that auto-requests every discovered Identity scope with exact MCP configuration and scoped login guidance.
relations:
- derived_from: epic:authenticated-control-plane
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/features/publications/PublicationsView.vue
- confidence: cited
  path: frontend/src/styles/main.css
revision: 6
---
## Outcome

An engineer can configure a Devcenter capability publication in Codex without the add-time OAuth
flow requesting unrelated Identity, Connector, Secrets, or Substrate authority.

## Acceptance

- The Publications view provides the exact streamable-HTTP URL, pre-registered OAuth client,
  OAuth resource, and only `mcp.tools.call` as the persisted Codex scope.
- The Codex login instruction explicitly requests only `mcp.tools.call`.
- The view no longer emits `codex mcp add` because Codex 0.145 ignores configured scopes during
  add-time OAuth discovery and requests the authorization server's complete `scopes_supported` set.
- Browser regression coverage fails if the unsafe add-time command returns.
- The frontend checks and relevant browser test pass.

## Evidence

Observed against Codex CLI 0.145.0 on 2026-09-03: the generated command opened authorization with
all scopes advertised by Devcenter's shared Identity authorization server even though the protected
resource metadata and bearer challenge both name only `mcp.tools.call`. The corresponding Codex
source in `cli/src/mcp_cmd.rs` constructs add-time login with explicit and configured scopes set to
`None`, then uses discovered authorization-server scopes.
