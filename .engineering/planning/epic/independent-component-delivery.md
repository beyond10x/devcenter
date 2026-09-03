---
format: aep.planning-md/1
id: epic:independent-component-delivery
kind: epic
status: active
title: Release and deploy Devcenter components independently
summary: Compile repository-owned ESS release units into an exact downstream stack and reconcile only affected workloads.
relations:
- derived_from: initiative:engineer-journey
revision: 3
---
## Outcome

Devcenter and each dependency publish independently versioned release units, while a downstream
private stack selects exact immutable artifacts and deploys only the units whose desired state
changed.

## Acceptance

- Repository-owned ESS documents compile source, build, release, stack, environment, and deployment intent into deterministic artifacts.
- Public repositories contain no deployment-specific coordinates or credentials.
- A downstream reconciler proves that changing one selected component does not rebuild or redeploy unrelated components.

## Current state

The repository now owns the typed multi-output build graph and generated BuildKit and Mermaid
projections. Independent release units are modeled, but the current release workflow still publishes
them together and the private deployment still consumes the existing chart/values flow.
