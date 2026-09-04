---
format: aep.planning-md/1
id: story:starter-workflow-library
kind: story
status: implemented
title: Install the reusable starter Workflow library
summary: Let an authorized engineer install and inspect real published starter graphs from Devcenter.
relations:
- derived_from: epic:authenticated-control-plane
- depends_on: story:workflow-library-read-surface
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-connectors
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: deploy/charts/devcenter/Chart.yaml
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/api
- confidence: cited
  path: frontend/src/features/workflows
- confidence: cited
  path: openapi.json
revision: 6
---
## Outcome

The standalone Workflow page contains reusable Workflow records with active immutable revisions instead of explanatory copy over an empty store.

## Context

Devcenter can read the standalone Workflow service, but a newly deployed tenant has no definitions. The three generic analysis workflows currently exist only as Workspace project projections, so the standalone page has nothing to inspect.

## Acceptance

- The empty library offers an explicit authenticated action to install the generic Code review, Security review, and Reverse AEP + ESS starter definitions.
- Installation uses an exact Identity-exchanged `workflows.manage` grant and the official generated Workflow client; Devcenter stores no authority or credential bytes.
- Each starter becomes a Workflow aggregate with a validated, published, activated immutable graph.
- Installation is repeatable without duplicating an already-present starter name.
- Library reads remain `workflows.read`; installing does not occur as a side effect of GET.
- Frontend and BFF tests cover empty, installing, success, refusal, and graph inspection states.

## Out of Scope

Executing graphs, project-bound workflow ownership, arbitrary workflow authoring, and deployment-specific identities or values.

## Scope

The Workflow BFF routes/client adapter, public HTTP contract, Workflow library Vue surface, generated frontend schema, tests, release metadata, and dependency pins.
