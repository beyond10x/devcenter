---
format: aep.planning-md/1
id: review-result:projects-quota-chart-pass-1
kind: review-result
status: active
title: Adversarial review of hosted quota chart configuration
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
unit: Devcenter quota chart/docs/tests working diff against 003f038301d5448d0771342b52868dd268026f8b; scoped diff sha256 bc38111cd94e9a63f0123c53531d0a576e8cf6acddeb33fd552eaae0977a1e67
verdict: nothing found
cases: executed 0→0, red 0 (read-only review; no new suite run)
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: none

1. git --no-pager diff --stat

```console
$ git --no-pager diff --stat
 .engineering/planning/journal.jsonl                |  5 +++
 .../planning/story/projects-connection-recovery.md | 28 ++++++++++++++-
 README.md                                          | 23 ++++++++++++
 ci/check-chart-rollouts.sh                         | 42 ++++++++++++++++++++++
 deploy/charts/devcenter/templates/substrate.yaml   | 34 +++++++++++++++++-
 deploy/charts/devcenter/values.schema.json         | 24 ++++++++++++-
 deploy/charts/devcenter/values.yaml                |  9 +++++
 7 files changed, 162 insertions(+), 3 deletions(-)
```

This is the coordinator-owned working diff that was supplied for review, including excluded AEP mutations. My implementation/test delta is empty. The only file I wrote is this assigned scratch report. I did not edit production, tests, planning, or versions.

2. Cases added

None. This pass was explicitly requested as read-only initially, with tests permitted only if a defect needed reproduction. No such defect was identified, so no new assertion or suite run was introduced.

3. Existing runner records inspected

I read the coordinator's chart-quota-red.log, chart-quota-green.log, helm319-lint.log, and chart-quota-render.yaml. The red trace reaches the missing --project-quota-ids assertion and exits through its cleanup trap. The supplied final render contains the separate claim mount in both init and daemon, the quota range argument, and the intended capability posture. The lint output says:

```console
==> Linting deploy/charts/devcenter
[INFO] Chart.yaml: icon is recommended

1 chart(s) linted, 0 chart(s) failed
```

The coordinator reported that the full rollout script and Helm 3.19 lint pass. Those are coordinator runner records; I did not re-execute or reattribute them. The installed Helm 3.8 binary was not used.

4. Findings

Nothing found in this bounded chart/docs/test review. No finding table rows or origin-routing requests. This is a review result, not approval or proof of live filesystem capability.

5. What was inspected and not found broken

- StatefulSet compatibility: the diff changes pod-template volumes, mounts, arguments and security context. The existing state volumeClaimTemplates, serviceName, selector and pod management policy remain intact. The separate claim is referenced as an ordinary PVC volume, so this addition does not require mutating the StatefulSet's immutable claim-template list.
- Filesystem separation: durable state remains at /var/lib/substrate/state.sqlite on state; only /var/lib/substrate/workspaces is covered by workspace-data. Both the permission init and daemon receive the same nested mount. No subPath, hostPath, mount propagation, or host namespace was added.
- Permission init: the new commands set the selected filesystem root to mode 0700 and uid/gid 65532 without recursively changing copied contents. The init retains only its existing CHOWN/FOWNER capabilities and allowPrivilegeEscalation false. Its sh -ec sequence remains fail-fast across the newline before the new commands.
- Opt-in authority: the new default has no extra claim, quota flag or SYS_ADMIN. Quota enabled requires an existing claim and adds SYS_ADMIN/allowPrivilegeEscalation true only to the nonroot daemon; other daemon capabilities remain dropped. The source render retains runAsNonRoot, explicit uid/gid, and readOnlyRootFilesystem.
- Range schema: enabled is boolean; endpoints are integers in 1..4294967295. The template computes the inclusive span as signed 64-bit arithmetic and refuses spans below 128, including reversed ranges. The selected range is rendered as an ordinary quoted START-END argument.
- Render checks: the new assertions consume rendered Substrate manifests, require both workspace mounts and the exact range/capability posture, and exercise missing-claim, reversed-range and short-range refusals. They do not execute a quota syscall or prove runtime capability inheritance.
- Documentation: it names SYS_ADMIN and privilege escalation explicitly, preserves the default posture, requires the filesystem's quota proof, and explains full hidden-tree copy and synchronized rollback after new writes. It does not claim that mounting a new filesystem retrofits limits onto old unquotaed resources.

Runtime quota implementation, live migration, actual child capability behavior and new filesystem enforcement were expressly excluded from this pass. They remain separate coordinator/implementor verification work.

6. Paths written outside the assigned worktree

None. This pass wrote only .scratch/projects-recovery/chart-quota-adversary-report.md. No tests, background processes, temporary resources, or build outputs were created. Token/cost counters were not exposed.

7. Findings block

```findings
[]
```
