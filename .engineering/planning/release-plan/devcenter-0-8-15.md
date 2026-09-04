---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-15
kind: release-plan
status: active
title: Release DevCenter 0.8.15
summary: Publish and deploy the reusable starter Workflow library as DevCenter 0.8.15.
relations:
- delivers: story:starter-workflow-library
- supersedes: release-plan:devcenter-0-8-14
revision: 2
---
## Scope

Publish the starter-library installer, immutable graph presentation, client-only generated Workflow dependency, and exact Workflow 0.3.5 compatibility as DevCenter 0.8.15.

## Qualification

The repository frontend, targeted browser, Rust, chart, rollout, version-consistency, planning, and leak gates pass from the exact release revision. Workflow 0.3.5 and Service SDK 0.5.7 release gates pass first.

## Sequence

1. Release Service SDK 0.5.7 with scope-only projection visibility and client/host feature separation.
2. Release Workflow 0.3.5 from the SDK-pinned generated service.
3. Merge and tag DevCenter 0.8.15.
4. Update only the Workflow and Devcenter image locks plus Workflow Identity scopes in the private development deployment; do not rebuild the chart.
5. Deploy once and verify health, the empty-library install action, three visible definitions, and published graph detail.

## Rollout Strategy

Promote the independently released Workflow and Devcenter image units through the existing digest-pinned chart. Stop on failed rollout or authenticated Workflow behavior.

## Monitoring

Observe release workflow completion, Kubernetes rollout readiness, HTTP health, Workflow service readiness, and the authenticated library install/read path.

## Rollback

Restore the prior immutable Workflow and Devcenter image digests and Identity registry value in the private deployment if health or the authenticated Workflow path fails.

## Approvals

Protected repository gates authorize public releases; the private deployment merge policy authorizes the development rollout.

## Communications

Report exact releases, image digests, deployment pipeline, and live Workflow-library result.
