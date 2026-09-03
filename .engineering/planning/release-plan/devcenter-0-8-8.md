---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-8
kind: release-plan
status: active
title: Release DevCenter 0.8.8
summary: Publish the approved current-agent selector as patch release 0.8.8.
relations:
- delivers: story:current-agent-selector
revision: 2
---
# Release plan: DevCenter 0.8.8

## Outcome

Publish DevCenter 0.8.8 from the exact default-branch commit containing the approved current-agent selector.

## Contents

- Make the compact current-agent dropdown visually prominent and explicitly labeled.
- Preserve route-backed agent switching and add browser regression coverage.
- Keep governed agent removal out of this release while `dependency-blocker:agent-removal-api` remains open.

## Publication

- Align Cargo, frontend, OpenAPI, chart, composed Connectors, lockfile, and changelog versions at 0.8.8.
- Run the repository frontend, browser, Rust, chart, rollout, version-consistency, and leak gates.
- Commit and push the exact release revision to the default branch through `b10x-bot[bot]`.
- Create and push annotated tag `0.8.8` at that exact default-branch commit.
- Verify the tag-triggered release workflow reaches a successful terminal state.
