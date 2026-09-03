---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-14
kind: release-plan
status: implemented
title: Release DevCenter 0.8.14
summary: Publish the production CSP editor-style fix as patch release 0.8.14.
relations:
- delivers: story:production-editor-csp-styling
- supersedes: release-plan:devcenter-0-8-13
revision: 4
---
# Release plan: DevCenter 0.8.14

## Scope

Publish the production CSP nonce bridge and its hosted-workbench regression coverage as DevCenter 0.8.14. This supersedes earlier candidate plans after 0.8.12 and 0.8.13 were released concurrently.

## Qualification

The repository frontend, browser, Rust, chart, rollout, version-consistency, planning, and leak gates must pass from the exact release revision.

## Sequence

1. Merge the bot-authored release revision to the default branch.
2. Create the immutable 0.8.14 tag from that merge.
3. Wait for the release workflow to publish the server image and chart.
4. Update only the Devcenter application image digest in the private development deployment.
5. Verify rollout health, release headers, and authenticated editor syntax colors.

## Rollout Strategy

Deploy the single Devcenter release unit to the development environment and stop on failed health or browser verification.

## Monitoring

Observe release workflow completion, Kubernetes rollout readiness, HTTP health, CSP nonce delivery, and computed Monaco token colors.

## Rollback

Restore the prior immutable Devcenter image digest in the private deployment if health or browser verification fails.

## Approvals

The repository gate and protected default-branch merge policy gate publication; the private deployment merge policy gates rollout.

## Communications

Report the exact release, image digest, deployment pipeline, and live browser result to the requesting engineer.
