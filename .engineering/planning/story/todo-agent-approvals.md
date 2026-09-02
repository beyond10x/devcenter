---
format: aep.planning-md/1
id: story:todo-agent-approvals
kind: story
status: implemented
title: Approve agent Connector calls in Devcenter
summary: Inspect and resolve exact agent calls without exposing approval authority to the browser.
relations:
- derived_from: epic:authenticated-control-plane
revision: 4
---
# Story: Approve agent Connector calls in Devcenter

## Outcome

An authenticated person can inspect an exact Connector call suspended by an agent, approve or deny it in Devcenter, and see the same task continue without exposing approval authority to the browser.

## Context

Agent Platform now exposes attempt-bound pending approvals and resumes calls with one-use Connectors evidence. Devcenter owns the human-facing BFF and browser experience that exchanges a verified session for that narrow approval authority.

## Acceptance

- The BFF allowlists task approval listing and resolution; tenant and actor remain server-derived.
- Approval issuance uses an ephemeral `connectors.approvals.issue` token obtained from Identity and never returns that token or the one-use proof to the browser.
- The BFF issues proof for the exact operation, connection, description lease, input, and owner context reported by Agent Platform, then hands it directly to the suspended task.
- The agent view renders pending input and supports explicit approve and deny decisions while the task stream remains connected.
- Rust, frontend, browser, chart, version, and leak gates pass.
- No realm appears in a route, payload, header, or client argument.

## Out of Scope

Todo domain behavior and persistence remain generated service concerns; deployment-specific grants and artifact locks remain downstream.

## Open Questions

None.
