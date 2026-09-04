---
format: aep.planning-md/1
id: story:restore-hosted-slack-connection
kind: story
status: active
title: Restore the hosted Slack connection
summary: Activate and verify principal-owned Slack OAuth through hosted Connectors.
relations:
- derived_from: epic:authenticated-control-plane
scope:
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/src/features/connections
revision: 4
---
## Outcome

Slack is a visible, guided, principal-owned connection in Devcenter rather than a curated card with an unexplained deployment-managed dead end.

## Context

Hosted Connectors already contains the Slack OAuth and read-only datasource implementation, but the development deployment publishes no Slack setup profile because its deployment-owned application policy and client secret are absent. The product also does not explain this unavailable state to engineers.

## Acceptance

Before activation the curated card names the missing administrator setup; after the deployment owner supplies the Slack application identifier and stores its secret through the protected Connectors admin API, an authenticated engineer can click Connect Slack, complete OAuth, see the principal-owned connection as callable, and read an admitted public, private, direct, or group-direct conversation without Devcenter receiving credential bytes.

## Operator boundary

Application registration, workspace selection, and the one client secret remain deployment operations in the private deployment repository and encrypted Secrets custody. Devcenter owns the actionable state and connection journey, not those values.

## Scope

The hosted Connectors Slack composition boundary, Devcenter's curated Connections projection, the generic chart configuration seam, and downstream environment policy and smoke verification.
