---
format: aep.planning-md/1
id: story:global-search-hotkeys
kind: story
status: implemented
title: Search and navigate Devcenter from anywhere
summary: Add authority-bounded Search all, mnemonic section hotkeys, and discoverable keyboard help.
relations:
- derived_from: initiative:engineer-journey
revision: 4
---
## Outcome

An authenticated person can find every readable Devcenter destination and navigate major sections without leaving the keyboard.

## Acceptance

- A shell-level Search all trigger opens with Ctrl/Command K and renders an accessible grouped combobox over sections, agents, repositories/projects, connections, Connector catalog, capability profiles, generated services, MCP publications, and documentation.
- Search fans out only through existing authority-checked read APIs, ranks exact and prefix matches before token and substring matches, bounds each group, debounces remote sources, rejects stale responses, and preserves usable partial results with named failures.
- Selecting a result only navigates. Existing records receive stable deep links; unopened repositories lead to a populated Projects search and still require an explicit Open action.
- One declarative registry owns Ctrl/Command K, ?, G then P/A/C/S/F/M/D, Escape behavior, visible hints, and the keyboard-help panel.
- Unmodified shortcuts never fire from editable controls or beneath a modal; the G chord times out and gives a visible continuation hint.
- Desktop, mobile, unit, keyboard, focus-management, accessibility, and production-build gates pass.
