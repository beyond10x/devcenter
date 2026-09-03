---
format: aep.planning-md/1
id: story:ghostty-live-theme-refresh
kind: story
status: implemented
title: Refresh the hosted terminal palette live
summary: Recreate Ghostty and replay retained output when the active Devcenter theme changes.
relations:
- derived_from: story:themes-dark-mode
scope:
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/review-e2e/hosted-workbench.spec.ts
- confidence: cited
  path: frontend/src/features/workbench/HostedTerminal.vue
revision: 5
---
## Outcome

An open hosted terminal adopts the active Devcenter palette immediately, without a page reload or loss of the underlying Workspace terminal session.

## Context

Review of the superseded pull request #26 found that Ghostty currently reads the theme only when its renderer first mounts. The previously tested recreation-and-replay behavior never reached the default branch.

## Acceptance

- Changing the effective `data-theme` while a terminal is open disposes and recreates only the browser renderer with the new terminal palette.
- Devcenter reconnects to the same Workspace terminal, replays retained output from the beginning, and returns to the running state without duplicate input, resize, socket, observer, or reconnect handlers.
- Component disposal tears down the theme observer and every renderer subscription.
- Browser coverage observes real ANSI foreground pixels before the change, the selected background palette after it, a second terminal attachment, and no page errors.

## Scope

- `frontend/src/features/workbench/HostedTerminal.vue`
- `frontend/e2e/devcenter.spec.ts`
- `frontend/review-e2e/hosted-workbench.spec.ts`

## Out of Scope

Dependency updates, release metadata, backend sealing, OpenAPI changes, editor styling, and every other change from pull request #26.
