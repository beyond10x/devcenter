---
format: aep.planning-md/1
id: dependency-blocker:service-sdk-resource-reconciliation
kind: dependency-blocker
status: open
title: Service SDK lacks immutable resource reconciliation
summary: The Workflow starter library cannot become a service-owned resource until Service SDK exposes tenant-neutral reconciliation.
owner: workflow
relations:
- blocks: story:restore-workflow-library
withholds: test_result
revision: 1
---
## Missing contract

Devcenter can stop sending an invalid Workflow page size immediately, but automatic availability of the immutable starter library depends on a released Service SDK capability that does not yet exist.

Service SDK 0.5.7 rejects a package `resources` field, exposes no resource obligation in its canonical service definition, and generates no idempotent pre-readiness resource reconciler. Workflow storage is tenant-partitioned, so an application-owned seed would invent deployment identity and duplicate SDK deployment behavior.

## Exit condition

Release a Service SDK resource contract with canonical bundle IR, stable identity and digest, tenant-neutral service visibility, idempotent pre-readiness reconciliation, and initialize-twice conformance. Then consume it in Workflow and replace Devcenter's temporary starter installer with the released Workflow resource.
