---
format: aep.planning-md/1
id: review-result:projects-quota-cutover-pass-1
kind: review-result
status: active
title: Storage cutover chart independent review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
unit: Devcenter chart 0.8.24 cutover change on fix/projects-quota-cutover, dirty tree based on 0915d94a293340921989f701f7f917757d75cef4; scoped diff sha256 665880e535fc0f95ced8b34d7d7127f2b4886c3423203dedbd12d8f5189a5768
verdict: nothing found
cases: executed 0→0, red 0 (read-only review; no new runner execution)
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: immutable chart delivery, complete private maintenance/resume values and live serialized cutover remain separately assigned

1. `git --no-pager diff --stat`

```console
 .engineering/planning/journal.jsonl                |  4 +++
 .../planning/story/projects-connection-recovery.md | 14 ++++++--
 README.md                                          | 21 ++++++++----
 ci/check-chart-rollouts.sh                         | 37 ++++++++++++++++++++++
 deploy/charts/devcenter/Chart.yaml                 |  2 +-
 deploy/charts/devcenter/templates/components.yaml  |  2 +-
 deploy/charts/devcenter/templates/substrate.yaml   |  2 +-
 deploy/charts/devcenter/values.schema.json         | 11 ++++++-
 deploy/charts/devcenter/values.yaml                |  2 ++
 9 files changed, 82 insertions(+), 13 deletions(-)
```

All listed changes were supplied for review and remain coordinator-owned, including AEP. My source/test/planning delta is empty. The header hash covers the exact diff of README.md, ci/check-chart-rollouts.sh, Chart.yaml, templates/components.yaml, templates/substrate.yaml, values.schema.json and values.yaml under deploy/charts/devcenter, against the named HEAD. The only file written by this pass is this assigned scratch report.

2. Added cases

None. This was a read-only review of the bounded chart change and existing new rendered regressions. No source modification, rendering, build, test, deployment or data mutation was performed.

3. Retained runner records inspected

The coordinator's chart-cutover-red.log shows the stopped Workspace and Substrate renders still carrying replicas: 1 under the previous chart. Execution stops at the first new Workspace replicas: 0 assertion, followed by the EXIT cleanup trap:

```console
+ grep -Fxq '  replicas: 0'
```

The later Substrate assertion is not claimed to have executed in that red run. The final source retains both assertions. chart-cutover-green.log is empty; the coordinator reports the complete script passed. This pass did not infer an exit status from an empty file or reattribute that run. The retained chart-cutover-lint.log states:

```console
==> Linting deploy/charts/devcenter
[INFO] Chart.yaml: icon is recommended

1 chart(s) linted, 0 chart(s) failed
```

4. Findings

Nothing found in the assigned chart change.

5. Inspected behavior and limits

- Explicit zero: components.yaml:13 uses dig with a fallback only for a missing replicas key, so zero remains zero while omitted counts retain one. The existing enabled condition still controls resource inclusion; replicas: 0 does not turn a component off or delete its Service/PVC.
- Substrate count: values.yaml supplies default one; substrate.yaml:21 renders the selected count; schema restricts it to integer zero or one. Existing values files inherit one through normal chart defaults. The composed-component schema similarly rejects non-integer and negative explicit counts.
- Persistent resources: neither change modifies Services, volume mounts, claim names, claim templates, PVC retention or controller selectors. Substrate's zero count changes a mutable StatefulSet field. Its quota executable and security posture are unchanged. The stopped-render tests require both zero-count controllers, the Workspace PVC and mount, Substrate state claim template and new workspace claim, and both Services.
- Test targeting: resource_manifest selects a complete rendered document by kind and metadata.name. Assertions consume the actual rendered Workspace Deployment and Substrate StatefulSet, not template-source strings. Default counts remain asserted as one, and Substrate negative/two-replica inputs must fail rendering. Existing chart checks remain intact.
- Atomic cutover: README.md:168–182 now requires an initial writer stop and verified complete copy, a successful Helm revision selecting the final filesystem/image/command with both counts zero, an explicit absence-of-writer-pods check, and a serialized second upgrade restoring one. The immediately preceding successful revision then contains the same current filesystem and state and can stop writers on automatic rollback. Pending or terminating pods must actually disappear; a desired zero count alone is not that observation.
- Storage guarantees: documentation now requires enforced quota storage to survive rollback while any quota-bound workspace survives. Returning to an ordinary old mount requires proving all surviving storage contracts still hold, as well as synchronized bytes/state. It does not retrofit quota identities onto preexisting unquotaed workspaces.

The private operator still must prepare complete effective maintenance/resume values with identical final image/claim pins, serialize the two stages against other deployments, verify the successful stopped Helm revision before resuming, and perform actual containerd/quota and coding-session checks. This chart review does not claim those operations have occurred. The new schema/templates alone do not transform the private repository's previous one-stage CI command into that procedure.

6. Outside-worktree writes

None for this chart pass. Only .scratch/projects-recovery/chart-cutover-review.md was written. The separately requested private deployment report is in that task's separately assigned scratch directory. No processes or temporary resources were created. No costs were exposed.

7. Findings block

```findings
[]
```
