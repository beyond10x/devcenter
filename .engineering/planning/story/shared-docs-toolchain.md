---
format: aep.planning-md/1
id: story:shared-docs-toolchain
kind: story
status: draft
title: Align the passive producer with the shared contract viewer runtime
scope:
- confidence: cited
  path: .github/workflows/b10x-docs-bundle.yml
revision: 2
---
## Outcome

Use the reviewed shared documentation runtime for this repository's passive source bundle producer as part of the coordinated Mandate contract viewer publication.

## Acceptance

The Atlas-generated producer caller pins Docs System commit 1d4c0262911761118ffdd7037890f541a0688714. Source allowlists, credential-free execution and repository-owned validation remain intact. Atlas reconciliation reports no caller drift. Required source and common checks pass on the exact published commit, and its passive producer yields a valid immutable source bundle.

## Scope

.github/workflows/b10x-docs-bundle.yml and this planning record. The coordinating authority is the shared documentation rollout; this repository's product/runtime contracts do not change.
