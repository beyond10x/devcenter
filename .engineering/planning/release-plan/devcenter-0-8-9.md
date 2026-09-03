---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-9
kind: release-plan
status: active
title: Release DevCenter 0.8.9
summary: Publish the current compatible component graph as DevCenter 0.8.9.
relations:
- delivers: story:align-current-released-components
revision: 2
---
# Release plan: DevCenter 0.8.9

## Outcome

Publish DevCenter 0.8.9 from the exact default-branch commit that composes the current compatible
released dependency graph.

## Contents

- Upgrade Agent Platform, AgentIDE, Workspace, Connectors, Service SDK, and Todo together.
- Preserve Identity at current release 0.5.6 and Eventlog at the Service SDK-selected revision.
- Adapt DevCenter to any changed sealed contracts and retain all security boundaries.
- Record Workflow's missing client contract without claiming unsupported integration.

## Publication

- Align Cargo, frontend, OpenAPI, chart, composed Connectors, lockfile, and changelog versions at 0.8.9.
- Run the repository frontend, browser, Rust, chart, rollout, version-consistency, and leak gates.
- Commit and push the exact release revision to the default branch through the required bot identity.
- Create and push annotated tag `0.8.9` at that exact default-branch commit.
- Verify the tag-triggered release workflow reaches a successful terminal state.
