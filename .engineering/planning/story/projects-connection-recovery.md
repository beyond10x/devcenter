---
format: aep.planning-md/1
id: story:projects-connection-recovery
kind: story
status: active
title: Restore Projects repository loading and connection recovery
relations:
- derived_from: initiative:engineer-journey
- informed_by: story:legacy-gitlab-startup-alignment
scope:
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/features/projects/ProjectsView.vue
revision: 6
---
## Outcome

Restore the Projects repository listing when current GitLab authority admits no usable connection, and provide a visible connection recovery action without misreporting an engineering-plan failure. O1 authority remains enforced and O4 repository navigation becomes operable.

## Evidence and scope

The authenticated repository endpoint returned 502 while session and agent requests succeeded. In the released implementation, GitLab Describe returns NotFound when no connection supports gitlab-project-list; Workspace maps that refusal to 502; the BFF uses a generic Workspace refusal code whose browser text incorrectly names an engineering plan. This code path is verified; whether it is the exact hosted cause remains under investigation.

- cited: frontend/src/features/projects/ProjectsView.vue, frontend/src/api/client.ts and frontend/e2e/devcenter.spec.ts.
- inferred: an upstream Workspace repository-search correction and a released client/runtime pin if required.

## Acceptance

A missing usable GitLab connection renders a clear recoverable Projects state. An admitted connected user sees repositories and can open an existing project. Real service failures remain distinguishable and are not silently returned as empty results. Regression tests reproduce the current failure before changes. Deployed authenticated browser verification records the actual endpoint results and rendered state; pod readiness alone cannot close this story.

## Delivery

One repair unit, an independent adversarial review, per-step repository gates, bot-authored integration and publication, immutable deployment and final browser validation. The operator authorized execution and deployment in the session. Preserve unrelated work and remove only this task's managed trees and named scratch resources after publishing evidence.
