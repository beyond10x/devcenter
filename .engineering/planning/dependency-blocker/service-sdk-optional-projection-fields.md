---
format: aep.planning-md/1
id: dependency-blocker:service-sdk-optional-projection-fields
kind: dependency-blocker
status: open
title: Service SDK rejects absent optional projection fields
summary: A populated Workflow library returns service_contract until Service SDK preserves optional view fields.
owner: service-sdk
relations:
- blocks: story:restore-workflow-library
withholds: test_result
revision: 1
---
## Proven failure

Workflow 0.3.5 at commit `f2bc043` returns HTTP 200 for `list_workflows` against an empty store. After a successful `create_workflow`, the same image and valid authority return HTTP 500 `service_contract`. A store created by Workflow 0.3.4 and opened by 0.3.5 behaves identically, excluding chart wiring, client/server skew, pagination, and database-version migration.

`WorkflowRow.active_revision_id` is optional. Service SDK 0.5.7 projection materialization omits the absent value, but projection validation still requires the row to contain every declared view field. The valid row is rejected as `InvalidProjection`, which the generated HTTP boundary returns as `service_contract` and Devcenter maps to `workflow_request_refused`.

## Exit condition

Service SDK preserves optionality in its realization contract and accepts an absent optional projection field while continuing to reject absent required fields. Existing stored rows work without reseeding. Workflow is regenerated and released against that SDK, then Devcenter pins the released Workflow commit. A real generated-service regression proves empty and populated library queries.

## Diagnostic hygiene

The diagnostic used temporary `/tmp/devcenter-workflow-*` container state outside the assigned scratch directory and removed it before returning. No repository files or persistent external resources were changed.
