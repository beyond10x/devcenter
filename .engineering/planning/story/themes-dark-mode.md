---
format: aep.planning-md/1
id: story:themes-dark-mode
kind: story
status: proposed
title: Add themes and dark mode
summary: Let people choose a persistent accessible Devcenter theme, including system-aware dark mode.
relations:
- derived_from: initiative:engineer-journey
revision: 2
---
## Outcome

Devcenter offers a coherent theme system with light, dark, and system preferences across every product surface, including embedded generated-service consoles.

## Context

The current frontend assumes a single light palette. Theme choice should be a first-class product preference built from semantic design tokens, not a second set of page-specific overrides.

## Acceptance

- A visible theme control offers Light, Dark, and System; System is the default and follows operating-system changes without a reload.
- An explicit choice persists locally before application paint so navigation and reloads do not flash the wrong theme.
- Semantic color, elevation, border, focus, code, status, chart, and overlay tokens drive all Devcenter views; components do not branch on theme.
- Generated service-console widgets inherit the host theme through documented tokens while standalone generated docs retain a usable default.
- Every theme maintains WCAG 2.2 AA contrast, visible keyboard focus, reduced-motion behavior, and browser-native control color-scheme.
- Frontend unit, browser, mobile, accessibility, and production-build gates cover all three preferences and both effective palettes.

## Out of Scope

Organization-specific branding, per-connector skins, and server-side storage of a browser display preference.
