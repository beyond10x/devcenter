---
format: aep.planning-md/1
id: story:vue-engineer-workspace
kind: story
status: implemented
title: Deliver the Vue engineer workspace
summary: Compose the authenticated engineer journey into a responsive, tested Vue product surface.
relations:
- derived_from: initiative:engineer-journey
revision: 4
---
## Outcome

An authenticated engineer can manage agents, run work, inspect ordered task output, and control a
user-owned model connection through a responsive Vue application rather than a static placeholder.

## Context

The BFF and service seams already expose the engineer journey, but the browser surface must compose
those seams into an operable product while keeping authority and credential custody visible.

## Acceptance

- The browser application is an independently built Vue and TypeScript workspace with generated
  OpenAPI types and no backend implementation mixed into its source tree.
- Signed-out, service-unavailable, agent-list, agent-create, task-submit, ordered task-event,
  connection-authorize, connection-revoke, and documentation journeys have explicit UI states.
- OAuth flow state remains in browser memory, one-time codes are cleared after submission, and no
  credential field or credential bytes enter Devcenter persistence or production assets.
- Rust embeds the production Vite output, serves only explicit application and immutable asset
  routes, and applies a strict Content Security Policy.
- Unit, browser, mobile, accessibility, Rust embedding, Helm, and confidential-marker gates pass.

## Out of Scope

Organization-specific deployment configuration, connector catalogues, arbitrary upstream proxying,
and ownership of provider credentials.

## Open Questions

None.
