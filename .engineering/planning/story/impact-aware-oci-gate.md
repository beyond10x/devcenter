---
format: aep.planning-md/1
id: story:impact-aware-oci-gate
kind: story
status: active
title: Build only affected OCI release units
summary: Derive OCI targets from changed repository surfaces and allocate no image-build runner when no release unit changed.
relations:
- decomposes: epic:independent-component-delivery
scope:
- confidence: cited
  path: .github/workflows/gate.yml
- confidence: cited
  path: Dockerfile.ess
- confidence: cited
  path: ci/check-release-unit-impact.sh
- confidence: cited
  path: ci/release-unit-impact.sh
- confidence: cited
  path: docs/ess-deployment-model.md
- confidence: cited
  path: ess/build.yaml
- confidence: cited
  path: generated/ess/build.json
- confidence: cited
  path: generated/ess/build.mmd
revision: 6
---
# Story: Build only affected OCI release units

## Outcome

An engineer changing one Devcenter release unit gets validation for the repository but pays the OCI build cost only for image units affected by that change.

## Context

The repository models server, Connectors, and deployment CLI as separate ESS build outputs, while the Gate workflow still builds all three after every qualifying pull request or default-branch push. Canceling redundant runs is an operational workaround, not impact-aware delivery.

## Acceptance

- Gate runs for pull requests, explicit reusable calls, or manual dispatches; ordinary feature/default-branch pushes do not create redundant workflow runs.
- A deterministic, locally testable classifier maps changed paths to `server`, `connectors`, and `deployment-cli` targets.
- Frontend or server changes select only `server`; Connectors changes select only `connectors`; deployment CLI changes select only `deployment-cli`.
- Shared build inputs select all image targets.
- Documentation and planning-only changes select no image target.
- The OCI job is skipped before runner allocation when the target set is empty.
- The server image target does not compile the deployment CLI, and the deployment CLI target does not build the frontend or server.
- Pull-request validation remains the stable merge gate; tag publication independently verifies the provider-reported default-branch tip.

## Out of Scope

Splitting the version namespace or tag-triggered release workflow into independently versioned publications, and changing the downstream private reconciler.

## Open Questions

None.
