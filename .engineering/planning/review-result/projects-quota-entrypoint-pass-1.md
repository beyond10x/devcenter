---
format: aep.planning-md/1
id: review-result:projects-quota-entrypoint-pass-1
kind: review-result
status: active
title: Independent review of hosted quota executable selection
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
unit: Devcenter quota executable chart follow-up on fix/projects-quota-entrypoint, dirty tree based on a8a5d983799b4fa7b1d686bc5bbcabcf8739d28c; scoped diff sha256 d4de26b09272d6eb9946cd1206e6a6ba58bbb13ae207685cecefbd13ae07fa1c
verdict: nothing found
cases: executed 0→0, red 0 (read-only review; no suite execution)
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: final Substrate image and hosted runtime validation remain separately assigned

1. `git --no-pager diff --stat a8a5d983799b4fa7b1d686bc5bbcabcf8739d28c`

```console
 .engineering/planning/journal.jsonl                         |  1 +
 .engineering/planning/story/projects-connection-recovery.md |  6 +++++-
 README.md                                                   | 11 +++++++----
 ci/check-chart-rollouts.sh                                  |  3 ++-
 deploy/charts/devcenter/Chart.yaml                          |  2 +-
 deploy/charts/devcenter/templates/substrate.yaml            |  3 +++
 6 files changed, 19 insertions(+), 7 deletions(-)
```

These are the changes supplied for review, including coordinator-owned AEP records. My source/test/planning delta is empty. The scoped hash in the header covers the diff of README.md, ci/check-chart-rollouts.sh, deploy/charts/devcenter/Chart.yaml and deploy/charts/devcenter/templates/substrate.yaml against the named base. The only file written by this pass is this assigned scratch report.

2. Cases added

None. This is the requested read-only review of an existing small chart change. I added no assertion, ran no tests or Helm commands, and made no deployment or image changes.

3. Retained runner records inspected

The coordinator supplied chart-quota-entrypoint-red.log, chart-quota-entrypoint-green.log and chart-quota-entrypoint-lint.log. The red trace renders the base chart's quota configuration, successfully checks --project-quota-ids, and then reaches the new exact command assertion while that command is absent. Its final assertion lines are:

```console
+ grep -Fq -- '- --project-quota-ids'
+ grep -Fq 'command: ["/usr/local/bin/substrate-daemon-quota"]'
```

The next line is the existing EXIT cleanup trap. This records a failure at the intended missing-command assertion, rather than an unrelated rendering error.

The complete green rollout log is empty; the coordinator reported the full rollout script, Helm lint and version checks passing. I did not reconstruct an exit code from the empty file or reattribute those supplied results to this pass. The retained lint output states:

```console
==> Linting deploy/charts/devcenter
[INFO] Chart.yaml: icon is recommended

1 chart(s) linted, 0 chart(s) failed
```

I also read the version checker: it treats chart version and application version as separate artifact versions. Chart 0.8.23 with unchanged appVersion 0.8.21 is consistent with that existing contract. No application source or image change is implied by this chart bump.

4. Findings

Nothing found in the bounded chart follow-up. This is a review result, not a claim that image xattrs, runtime capability sets or quota syscalls have been verified here.

5. Inspected behavior

- Default startup and authority: projectQuotas.enabled remains false by default and schema-typed as a boolean. The new command is entirely inside that existing condition. With quotas disabled the daemon still has no command override, keeps its image's ordinary entrypoint, drops ALL capabilities, sets allowPrivilegeEscalation false, and retains UID/GID 65532, runAsNonRoot and the read-only root filesystem. A separately supplied workspace claim does not itself enable the command or quota authority.
- Quota wiring: substrate.yaml:82 selects the exact /usr/local/bin/substrate-daemon-quota path on the daemon container, with no shell wrapper. Its existing args list remains separate and unchanged, including socket, state, workspace root, deployment, quota range and conditional Git source. The same quotas.enabled value selects the quota argument, SYS_ADMIN bounding delegation and privilege-escalation opt-in; there is no second switch that can silently diverge.
- Existing boundaries: required-claim and minimum-range validation, both workspace-data mounts, permission init, TLS/env configuration, resources and probes are unchanged. No capability, host mount, namespace, seccomp setting or persistent claim template was added by this follow-up. The StatefulSet change is confined to its pod template command, alongside the normal chart-version label change.
- Regression assertions: ci/check-chart-rollouts.sh:97 now excludes the quota executable from the default render; :107 requires the exact command in the quota render. Both operate on the rendered Substrate template. Existing capability, mount and invalid-configuration checks remain intact. These checks establish chart selection, not executable availability or actual process authority.
- Deployment documentation: README.md:158 names Substrate 0.7.4 or later and the exact quota executable, explains its cap_sys_admin=ep metadata, and requires the image/command pairing during deployment and rollback. It preserves the ordinary default and the separate requirements for child capability behavior and any terminal execution profile. The live repair must pair this chart option with the newly verified image; older images do not acquire the executable through a chart change.

Substrate packaging, final-image verification, containerd process/thread capability inspection, real enforced quota facts and private migration/deployment remain with their separately assigned implementor/coordinator work.

6. Paths written outside the assigned worktree

None. This pass wrote only .scratch/projects-recovery/chart-quota-entrypoint-review.md. It created no test/build outputs, temporary resources or background process. Source and test files were never changed. No token or cost counters were exposed.

7. Findings block

```findings
[]
```
