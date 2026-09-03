---
format: aep.planning-md/1
id: story:production-editor-csp-styling
kind: story
status: implemented
title: Preserve editor styling under production CSP
summary: Authorize Monaco runtime theme styles with per-response nonces and cover the deployed security-header path.
relations:
- derived_from: epic:authenticated-control-plane
scope:
- confidence: cited
  path: ci/check-chart-rollouts.sh
- confidence: cited
  path: crates/devcenter-http
- confidence: cited
  path: frontend/e2e
- confidence: cited
  path: frontend/src
revision: 7
---
# Story: Preserve editor styling under production CSP

## Outcome

An engineer opening a source file in the hosted workbench sees the selected Monaco syntax theme in the production deployment.

## Context

The application server enforced `style-src 'self'`, while Monaco creates token and theme rules in runtime style elements. Development-server browser tests did not send that production header, so the released editor became monochrome despite passing locally.

## Acceptance

- Every application document receives a fresh cryptographic CSP nonce.
- The bootstrap script and Monaco runtime styles carry the same nonce.
- Neither script nor style policy admits `unsafe-inline`.
- Browser coverage runs the hosted workbench under production-equivalent CSP and observes at least three computed token colors.
- The complete repository gate passes.

## Out of Scope

Changing Monaco, weakening CSP globally, or redesigning the AgentIDE renderer contract.

## Open Questions

None.
