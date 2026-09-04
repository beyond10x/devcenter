---
format: aep.planning-md/1
id: story:retained-oauth-continuation-link
kind: story
status: implemented
title: Retain an OAuth continuation link
summary: Popup-blocked connection sessions must retain an explicit safe authorization link.
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

The OAuth completion URL is only sent to `window.open`. A popup blocker can therefore strand a valid pending connect session with no action beyond waiting.

## Acceptance

While a browser connect session is pending, its validated completion URL remains available as an explicit user-initiated continuation link. The UI does not render unsafe URL schemes, and popup-blocked coverage proves that authorization remains recoverable.
