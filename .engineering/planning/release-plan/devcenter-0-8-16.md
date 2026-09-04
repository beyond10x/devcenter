---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-16
kind: release-plan
status: active
title: Release Devcenter 0.8.16
summary: Restore and qualify the four core authenticated engineer journeys before expanding the surface.
relations:
- delivers: story:restore-workflow-library
- delivers: story:complete-project-workflow-runs
- delivers: story:refresh-gitlab-authority
- delivers: story:refresh-user-bound-model-credential
- delivers: story:restore-hosted-slack-connection
- delivers: story:deploy-engineer-journey
- supersedes: release-plan:devcenter-0-8-15
revision: 3
---
## Release rule

This is a recovery release. It adds no unrelated product surface and is not qualified by pod readiness, a successful aggregate Helm apply, or an accepted asynchronous request.

## Ordered path

1. Preserve downstream refusal status and safe codes at each BFF boundary, then reproduce the Workflow, GitLab, and model failures with authenticated requests.
2. Restore the reusable Workflow library from a Workflow-owned immutable bundle and prove list plus graph inspection after a fresh deploy.
3. Add first-class connection lifecycle states and repair actions, then prove GitLab default-branch repository reads and model-backed terminal agent output.
4. Activate the existing Slack adapter from private deployment policy and Secrets custody, while making pre-activation administrator setup explicit in the UI.
5. Prove one project Workflow moves beyond accepted to exactly one observable terminal result, including a disruption/recovery check.
6. Promote only changed immutable component images/digests and run the four-journey authenticated smoke gate against the deployed revision.

## Qualification

The release is qualified only by recorded authenticated evidence for Workflow library read and project Workflow completion, GitLab reconnect plus project/file read, a model-backed agent terminal result, and Slack OAuth plus one read-only conversation query. Every component version and digest used by that evidence is captured with the result.

## Deployment model

Each owning repository releases its affected component independently. The private stack changes only selected immutable digests and deployment policy; it does not rebuild the aggregate chart or unrelated services. A failed journey blocks promotion even when every workload is Ready.

## Deferred functionality

After this gate, the next slices are capability-profile lifecycle and connection testing, usable MCP publication/client onboarding, deployment-managed Grafana/Prometheus access, and Workflow/AEP composition. None of them may mask or postpone the recovery gate.

## Execution

The operator authorized the full release and deployment chain on 2026-09-04 and required every wave worktree to be integrated into main and retired. Devcenter 0.8.16 consumes Service SDK 0.5.8 at f57255a2886cae3ace2a3a35935e8f1fd91a5fd4, Workflow 0.3.6 at 77c204b4d7a913cdc26e7d3445229571bad84f9a, and Workspace 0.2.15 at 9ec16c442867121c2add71ce2922af1223e57969. Promotion remains contingent on immutable release artifacts, repository gates, downstream render validation, and authenticated live qualification.
