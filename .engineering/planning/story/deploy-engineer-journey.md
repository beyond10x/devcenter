---
format: aep.planning-md/1
id: story:deploy-engineer-journey
kind: story
status: active
title: Deploy and verify the engineer journey
summary: Run all released components in the devcenter namespace and verify the complete internal experience.
relations:
- derived_from: initiative:engineer-journey
scope:
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: deploy/charts/devcenter
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src/features/connections
- confidence: cited
  path: frontend/src/features/projects
- confidence: cited
  path: frontend/src/features/workflows
revision: 5
---
## Outcome

An engineer can complete the four core authenticated journeys—repository access, model-backed agent work, Workflow use, and Slack connection—against the deployed product without operator database edits or hidden recovery commands.

## Context

The current deployment demonstrates why workload readiness is insufficient: every product pod is Ready while Workflow returns `workflow_request_refused`, GitLab projects cannot be read, the model connection is reported as present but cannot fund a turn, and Slack has no actionable setup. This story is the product-level release gate over the component stories; it does not absorb their implementations.

## Acceptance

From one fresh Identity session in the development environment, an engineer can reconnect and read a GitLab project, start a model-backed agent turn and receive terminal output, list and inspect the reusable Workflow library and complete one project workflow run, and connect Slack from the curated Connections surface; the same authenticated smoke suite fails deployment promotion if any journey regresses.

## Verification boundary

Health probes remain rollout prerequisites, but only the authenticated smoke suite qualifies the release. Each failure must retain the downstream service, HTTP status, and safe refusal code so a generic `*_request_refused` message is never the sole diagnostic.

## Scope

Devcenter's authenticated smoke harness, allowlisted BFF routes, browser recovery actions, component pins, and downstream deployment qualification contract.
