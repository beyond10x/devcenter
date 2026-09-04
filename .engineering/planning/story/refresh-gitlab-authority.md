---
format: aep.planning-md/1
id: story:refresh-gitlab-authority
kind: story
status: active
title: Refresh GitLab authority without rebuilding Devcenter
summary: Promote the fixed Connectors runtime as an independent Devcenter release unit and restore live repository reads.
relations:
- decomposes: epic:independent-component-delivery
scope:
- confidence: cited
  path: .github/workflows/promote-connectors.yml
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src/features/connections
- confidence: cited
  path: frontend/src/features/projects
revision: 8
---
## Outcome

An engineer can understand and repair a GitLab connection from Devcenter, and its refreshed grant keeps repository discovery and file reads working without rebuilding unrelated components.

## Context

The live curated card treats any non-revoked GitLab connection as “Connected,” including degraded or unusable authority, and offers no replace, reconnect, revoke, or access-check action. A stored connection can therefore block the only visible recovery path while Projects fails elsewhere.

## Acceptance

An authenticated engineer can see whether the GitLab connection is callable or needs attention, replace or revoke it through Connections, complete OAuth again, and immediately list projects and read the selected repository's default branch; a later OAuth refresh preserves the admitted repository scope and deploys as only the affected Connectors/Devcenter units.

## Out of Scope

Changing repository visibility, accepting the upstream GitLab token as a Devcenter bearer, widening the engineer's upstream grant, or assuming a branch named `main`.

## Scope

The hosted Connector lifecycle API, Devcenter's allowlisted connection routes, the curated and generic Connections UI, Projects recovery guidance, independent Connectors promotion, and authenticated repository smoke checks.
