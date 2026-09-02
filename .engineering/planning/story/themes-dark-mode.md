---
format: aep.planning-md/1
id: story:themes-dark-mode
kind: story
status: implemented
title: Add an accessible multi-theme design system
summary: Offer persistent system, Devcenter, Monokai, and Solarized palettes across every product surface.
relations:
- derived_from: initiative:engineer-journey
revision: 6
---
## Outcome

Devcenter offers one coherent semantic design system with six accessible flat presets across authenticated, public, and embedded generated-service surfaces.

## Acceptance

- A visible control offers System, Light, Dark, Monokai, Solarized Light, and Solarized Dark; System is the default and follows operating-system changes without reload.
- An explicit allowlisted preference persists locally and is applied before Vue mounts, including `data-theme`, `data-theme-preference`, native `color-scheme`, and browser theme color, without a wrong-theme flash.
- Semantic canvas, surface, text, border, accent, status, focus, overlay, shadow, chart, and code tokens drive all Devcenter views; components do not branch on named themes.
- Monokai is a complete application palette. Solarized Light and Dark remain recognizable while foreground/background pairs satisfy WCAG 2.2 AA.
- The exact released Service SDK widget inherits the documented `--b10x-*` host-token contract while standalone generated docs retain accessible system-aware fallbacks.
- Unit, browser, mobile, accessibility, representative visual-regression, and production-build gates cover all presets and both System resolutions.

## Out of Scope

Organization branding, per-connector skins, custom palettes, and server-side or synchronized display preferences.
