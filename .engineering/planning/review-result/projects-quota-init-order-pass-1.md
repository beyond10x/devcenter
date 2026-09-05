---
format: aep.planning-md/1
id: review-result:projects-quota-init-order-pass-1
kind: review-result
status: active
title: Nested quota volume initialization review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
unit: Chart 0.8.25 nested volume initialization on fix/substrate-volume-init-order, based on 54ce43058b333213d7fa8d4158919ee4628037c3; combined scoped patch SHA-256 bfbbd5802091ffa4cd9824840d011ab60b9ff2f586f550c64ccda47a40fbcb9d
verdict: nothing found
cases: executed 0→0, red 0 by this reviewer (read-only); retained implementor lane executes 8→8, failures 4→0
origin: introduced 0 / pre-existing 0 / undecided 0
wrote-outside-worktree: none
needs-coordinator: commit the new helper with the tracked changes, complete required gates and immutable chart release, then prove the exact hosted initializer and staged cutover separately

1. Scoped diff and untracked helper

`git --no-pager diff --stat 54ce43058b333213d7fa8d4158919ee4628037c3 -- README.md ci/check-chart-rollouts.sh deploy/charts/devcenter/Chart.yaml deploy/charts/devcenter/templates/substrate.yaml`:

```console
 README.md                                        | 4 ++++
 ci/check-chart-rollouts.sh                       | 7 +++++--
 deploy/charts/devcenter/Chart.yaml               | 2 +-
 deploy/charts/devcenter/templates/substrate.yaml | 7 ++++---
 4 files changed, 14 insertions(+), 6 deletions(-)
```

`git --no-pager diff --no-index --stat /dev/null ci/check-substrate-volume-permissions.sh` additionally reports one new 130-line file. That helper was still untracked during review; ordinary git diff does not include it. These five paths are implementor/coordinator changes. My only write is this assigned report. Coordinator-owned AEP changes are excluded from the implementation hash and were not edited.

Exact hash recipe for the combined five-file patch, including the new helper's patch and file mode:

```bash
{
  git diff --binary 54ce43058b333213d7fa8d4158919ee4628037c3 -- \
    README.md ci/check-chart-rollouts.sh deploy/charts/devcenter/Chart.yaml \
    deploy/charts/devcenter/templates/substrate.yaml
  git diff --no-index --binary /dev/null ci/check-substrate-volume-permissions.sh
} | sha256sum
```

The no-index diff intentionally returns 1 for the new file; its patch bytes are included. Combined hash: `bfbbd5802091ffa4cd9824840d011ab60b9ff2f586f550c64ccda47a40fbcb9d`. Tracked four-file diff alone: `9af7cc4de66030e4e61367af16876be26f56d1e4caf0a02f689a6e4a4e49e3a2`. Raw helper bytes: `71aff15e631f0e9fa8c2fceedcdb5e9b26eea10865b4c66fb13b3dc96e40e043`.

2. Read-only scope and runner records

No cases were added or executed. I read the complete five-file change, full Substrate template, new helper, relevant chart values, existing rollout checks, both CI call sites, the story's Nested volume initialization acceptance text, implementor report and retained focused/full chart logs. No Rust compilation, Docker invocation, Helm rendering, integration tool or deployment was performed by this reviewer.

The retained `chart-init-order-red.log` shows shared layout passing for both initial state-parent owners and both runs. All four separate-layout runs fail with the specific missing parent traversal access:

```text
chmod: cannot access '/var/lib/substrate/workspaces': Permission denied
FAIL: separate initial-owner=0 run=1
chmod: cannot access '/var/lib/substrate/workspaces': Permission denied
FAIL: separate initial-owner=0 run=2
chmod: cannot access '/var/lib/substrate/workspaces': Permission denied
FAIL: separate initial-owner=65532 run=1
chmod: cannot access '/var/lib/substrate/workspaces': Permission denied
FAIL: separate initial-owner=65532 run=2
volume permissions: executed 8, passed 4, failed 4
```

Both `chart-init-order-green.log` and `chart-init-order-rollouts-green.log` contain all eight named PASS results, ending:

```text
volume permissions: executed 8, passed 8, failed 0
```

The retained Helm record identifies v3.19.0+g3d8990f; lint reports one chart linted and zero failed. The implementor records successful version, shell-syntax and diff checks, and explains the intermediate full-rollout failure caused by existing assertions matching the old literal chown argument lists. The final assertions retain the required ownership destinations and require the final state handoff. No runtime case was removed or weakened to produce green. These are retained runner records, not executions or hosted observations by this reviewer.

3. Runtime and authority assessment

The original failure is reachable whenever the initializer must resolve `/var/lib/substrate/workspaces` after the mode-0700 state parent belongs to UID 65532. The root initializer has CHOWN and FOWNER, which suffice for the ownership/mode operations but do not bypass directory search permission through that private parent. This applies on an ordinary restart as well as a new separate-mount setup.

The new first command takes ownership of the state mount root and the already-handled TLS destination root. The initializer then establishes 0700 modes, refreshes the two TLS destination files and their required modes/ownership, and initializes the separate workspace mount while the state parent is still root-owned and traversable. It returns runtime/TLS directory ownership next and the state parent last. The `&&` chain ends in that final chown for both shared and separate layouts. An intermediate failure prevents the daemon from starting because the init container has not succeeded; another init attempt can retake the state/TLS parents and repeat the operation.

The patch does not add a capability or relax a mode. Init remains UID/GID 0 with only CHOWN/FOWNER, no privilege escalation and a read-only container root filesystem. The daemon remains UID/GID 65532 with the unchanged default or explicit quota capability profile. State and workspace claims, mount paths, StatefulSet identity, replica behavior, quota entrypoint, quota range, TLS source selection and immutable image handling are unchanged. There is no recursive chown or chmod across stored workspace files. The original state data and nested workspace contents are not copied, deleted or rewritten by this initializer; only the existing TLS refresh replaces its two fixed destination files.

In the default shared-state layout, no new operation traverses the workspace child. The state root is handed back before the daemon starts and can create/open its workspace directory as before. The separate layout performs the necessary workspace-root ownership/mode changes before that handoff. Temporarily taking state-root ownership is compatible with the single-daemon StatefulSet/init lifecycle and the prescribed stopped-writer cutover; it is not a live multi-writer migration mechanism.

4. Regression and integration assessment

The new helper renders the real Substrate template with Helm, isolates its volume-permissions initializer and extracts the literal shell block without rewriting it. It requires the expected image, shell arguments, root UID/GID, no privilege escalation, read-only root and exact CHOWN/FOWNER declaration. It runs that command in a disposable container using the pinned public PostgreSQL image with network disabled, read-only root, no-new-privileges, only those two capabilities, private tmpfs data/runtime mounts and a read-only fixture TLS source. The separate case adds an actual nested tmpfs mount.

The setup checks permitted/effective/bounding mask 0x9, empty inheritable/ambient sets and NoNewPrivs=1, and checks root/TLS-source writes are refused. Existing state/workspace/hidden-baseline bytes and private TLS files are seeded before initialization. The matrix independently varies shared/separate layout and root/daemon initial state-parent ownership, then executes twice on the same volume state. Verification runs as UID/GID 65532 and checks exact directory/file ownership and modes, durable bytes, hidden bytes and refreshed TLS content. This is a useful execution regression for the DAC/traversal failure, not just an assertion that the template contains an intended string.

The final rollout script invokes the helper unconditionally through bash, so its current non-executable 0644 file mode is sufficient. Both the repository gate and chart-publication workflow already call that rollout script on an Ubuntu runner; the gate already provisions a Docker PostgreSQL service and the chart release sets up Docker. The added check consequently participates in both validation paths. It adds an explicit local Docker/image requirement to running the rollout script; it does not silently skip without Docker. The image is a content-addressed public fixture, and its placeholder certificate/key text is not used as an application credential.

Cleanup removes only the named disposable container with its anonymous volumes and the helper's unique temporary TLS directory. The retained cleanup log reports no remaining fixture containers and names the reused image and twelve removed container IDs. I did not re-query Docker to turn that historical record into a current cleanup claim. The helper creates no persistent named volume, build, external service message or production resource.

These tests establish the rendered command's permission behavior under the exercised local Docker profile. They do not establish current Kubernetes PVC contents/mount topology, kernel project-quota operation, actual hosted daemon capability masks, TLS validity, a successful Helm rollback, or the coding-session journey. The coordinator's exact hosted initialization and staged cutover checks remain necessary. The public README's 0700/0644/0600 and CHOWN/FOWNER description matches the scoped initializer change. Chart version advances to 0.8.25 while appVersion stays 0.8.21.

5. Outside-worktree writes and findings

None. Only `.scratch/projects-recovery/chart-init-order-review.md` was written. Production source, tests, planning files, credentials, scripts, images, containers, managed trees and cluster resources were not modified or removed by this reviewer. No costs were exposed.

```findings
[]
```
