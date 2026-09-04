---
format: aep.planning-md/1
id: story:promote-workflow-runtime
kind: story
status: active
title: Promote the standalone Workflow runtime
summary: Publish and consume Workflow as an independently pinned runtime while restoring the qualified Workspace/AEP graph.
relations:
- derived_from: epic:independent-component-delivery
- depends_on: story:workflow-library-read-surface
scope:
- confidence: cited
  path: .github/workflows/promote-workflow.yml
revision: 4
---
# Story: Promote the standalone Workflow runtime

## Outcome

A released Workflow source revision can be promoted independently into one signed, immutable runtime image that the generic chart can consume without rebuilding Devcenter.

## Context

The Workflow client and chart seam are released, but the composition repository has no promotion path for the standalone server artifact. A downstream stack can therefore enable the Workflow UI without having a deployable Workflow image.

## Acceptance

- Promotion accepts an exact semantic version and peeled Workflow release commit, then verifies both against the upstream tag and manifest.
- The amd64 and arm64 images are built once from that source, assembled into an immutable multi-platform manifest, signed, and kept in the private composition package.
- The Workflow release receives a credential-free manifest naming the source commit and image digest.
- A downstream stack can pin and verify the Workflow runtime independently, require the component during deployment validation, and expose the library only when the workload is ready.
- The deployed Workspace runtime is aligned with the AEP client release against which the current Devcenter graph was qualified.

## Out of Scope

Workflow execution semantics, accepting arbitrary service origins, or embedding deployment-specific values in this repository.

## Open Questions

None.
