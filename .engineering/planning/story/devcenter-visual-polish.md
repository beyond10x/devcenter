---
format: aep.planning-md/1
id: story:devcenter-visual-polish
kind: story
status: implemented
title: Raise Devcenter visual quality
summary: Apply a focused semantic canvas, depth, navigation, status, empty-state, and typography pass.
relations:
- derived_from: initiative:engineer-journey
- depends_on: story:themes-dark-mode
revision: 4
---
## Outcome

Devcenter feels visually deliberate and coherent across its existing workflows without adding ornamental complexity or changing domain behavior.

## Acceptance

- Every theme supplies a restrained ambient canvas and the same semantic panel/card depth system.
- Primary navigation has a stronger active rail, themed icon wells, and useful status/count badges without relying on color alone.
- Connected, running, approval, failed, and disabled states use one icon-label-tint language.
- Empty states use one accessible icon-halo, explanation, and primary-next-action pattern.
- Typography has consistent heading rhythm, tabular numeric data, quiet monospaced identifier pills, truncation, and copy affordances.
- The pass does not broaden into skeleton loaders, general motion, section-hero redesigns, or unrelated dialog rewrites.
- Desktop, mobile, accessibility, representative visual-regression, and production-build gates pass.
