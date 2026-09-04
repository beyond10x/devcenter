---
format: aep.planning-md/1
id: access-blocker:operator-authenticated-activation
kind: access-blocker
status: open
title: Operator-authenticated activation is still required
summary: Slack application registration and final authenticated Workflow and Slack acceptance require operator-owned sessions.
relations:
- blocks: story:restore-hosted-slack-connection
- blocks: release-plan:devcenter-0-8-15
- blocks: release-plan:devcenter-0-8-16
withholds: test_result
revision: 2
---
## Missing access

Slack activation needs one deployment-owned application registered in the intended Slack workspace. Its non-secret client identifier must enter private deployment policy, and its client secret must be supplied through the Connectors administrative stdin flow into encrypted Secrets custody.

No Workflow database seed or authenticated “install starter library” click is an acceptable clearing action; the Workflow-owned resource bundle must reconcile automatically.

## Evidence already observed

- The live Workflow process is Ready, but an authenticated library request is refused and readiness does not prove the journey.
- Hosted Connectors is Ready, but publishes no Slack setup profile because no Slack policy is configured.
- The prepared private Slack manifest matches the runtime callback shape and requested user scopes, but application registration and the resulting credentials remain operator-owned.

## Clearing condition

Clear this blocker after the Slack application identifier is deployed, the client secret is stored through the protected administrative API, and an authenticated engineer completes the visible Slack OAuth connection and one read-only conversation query.
