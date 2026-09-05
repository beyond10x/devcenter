---
format: aep.planning-md/1
id: story:projects-connection-recovery
kind: story
status: active
title: Restore Projects repository loading and connection recovery
relations:
- derived_from: initiative:engineer-journey
- informed_by: story:legacy-gitlab-startup-alignment
scope:
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: ci/check-chart-rollouts.sh
- confidence: inferred
  path: ci/check-substrate-volume-permissions.sh
- confidence: cited
  path: deploy/charts/devcenter/Chart.yaml
- confidence: inferred
  path: deploy/charts/devcenter/templates/components.yaml
- confidence: cited
  path: deploy/charts/devcenter/templates/substrate.yaml
- confidence: inferred
  path: deploy/charts/devcenter/values.schema.json
- confidence: inferred
  path: deploy/charts/devcenter/values.yaml
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/features/projects/ProjectsView.vue
- confidence: cited
  path: openapi.json
revision: 23
---
## Outcome

Restore the Projects repository listing when current GitLab authority admits no usable connection, and provide a visible connection recovery action without misreporting an engineering-plan failure. O1 authority remains enforced and O4 repository navigation becomes operable.

## Evidence and scope

The authenticated repository endpoint returned 502 while session and agent requests succeeded. In the released implementation, GitLab Describe returns NotFound when no connection supports gitlab-project-list; Workspace maps that refusal to 502; the BFF uses a generic Workspace refusal code whose browser text incorrectly names an engineering plan. This code path was confirmed as the hosted cause; a normal OAuth reconnect restored an admitted connection, without editing grant or credential records.

- cited: frontend/src/features/projects/ProjectsView.vue, frontend/src/api/client.ts and frontend/e2e/devcenter.spec.ts.
- inferred: an upstream Workspace repository-search correction and a released client/runtime pin if required.

## Acceptance

A missing usable GitLab connection renders a clear recoverable Projects state. An admitted connected user sees repositories and can open an existing project. Real service failures remain distinguishable and are not silently returned as empty results. Regression tests reproduce the current failure before changes. Deployed authenticated browser verification records the actual endpoint results and rendered state; pod readiness alone cannot close this story.

## Delivery

One repair unit, an independent adversarial review, per-step repository gates, bot-authored integration and publication, immutable deployment and final browser validation. The operator authorized execution and deployment in the session. Preserve unrelated work and remove only this task's managed trees and named scratch resources after publishing evidence.

## Hosted branch discovery finding

Authenticated repository discovery recovered after normal OAuth reconnect. Project details loaded, but branches took roughly 18 seconds and a subsequent branch selection returned 503 after timing out. Source inspection shows discover_branches resolves gitlab.branches bindings by scanning the provider membership project catalogue; each datasource page resolves the same binding through another scan. This measured UI path is in the original Projects loading scope.

The repair additionally uses the existing admitted gitlab-branch-list operation, bound to the already revalidated project and connection, with 100 records per provider page. It preserves the current Branch response and failure semantics, fetches additional pages until a short page, and never uses missing authorization as permission. Regression fixtures must prove exact numeric project identity and fresh description use, current connection admission, page progression, and errors instead of hidden partial lists. No cross-principal cache or new endpoint is introduced.

## Hosted workspace preparation finding

Authenticated post-deployment Projects and branch selection pass. The same browser then creates a coding session on the provider default branch; the workbench renders but preparation is refused. The Substrate durable operation is workspace.create with HTTP 501 and workspace.storage-quota-unserved: the active host has not proved hard workspace storage quotas. The deployed chart neither delegates project quota identifiers nor provisions a quota-capable workspace filesystem. The daemon is non-root with all Linux capabilities dropped. This is a deployment capability gap, after source authorization, and it must be resolved without removing the requested byte/inode ceiling or weakening runtime confinement.

The remaining authorized delivery includes the smallest generic chart configuration and downstream storage provisioning needed to serve the existing quota contract, preserving existing durable state and workspaces with a verified migration and rollback procedure. Investigation will establish filesystem and kernel privilege requirements before choosing implementation. Machine-readable implementation scope will be added once that option is concrete. Completion still requires a real hosted coding file tree and editable file, measured startup stages, independent review, immutable publication, deployment verification, and task cleanup.

## Hosted quota implementation scope

Provide an opt-in hosted quota configuration for the existing Substrate Git/file profile. The generic chart accepts a separately provisioned workspace PVC and an exclusive project-id range, mounts that PVC at the existing workspace root while preserving the original state volume and immutable StatefulSet claim template, and explicitly enables only the SYS_ADMIN capability required by quotactl_fd when project quotas are selected. The daemon remains non-root, drops all other capabilities, keeps a read-only root filesystem and mounts no host paths or namespaces. Kubernetes requires allowPrivilegeEscalation with SYS_ADMIN; this explicit opt-in must document that authority rather than pretend it remains false. No CAP_SYS_RESOURCE, privileged pod, namespace admission-policy relaxation or quota bypass is allowed. The default profile remains unprivileged.

Validate the range and required PVC at render time. Add positive/negative chart regression checks for quota arguments, the separate mount, default capabilities and invalid configurations. Update generic deployment documentation and publish only the changed chart output. The downstream operator must provision ext4 with project+quota features and enforced project quotas, freeze writers for an inventory/hash-verified complete workspace-tree migration, retain the original data and document a synchronized rollback. Existing eight workspace records have no storage quota; migration preserves that state and does not invent allocations.

A related Substrate source repair is owned by story:git-workspace-quota-lifecycle in its repository: attach quota before Git writes, preserve allocation on rename, kernel accounting on observation and complete failure/restart cleanup. Configuration alone cannot close this story. Real quota enforcement and the authenticated editor remain required validation. The current published runtime does not include the sandbox toolchain/cgroup delegation for terminal execution; this repair does not claim to introduce that separate serving profile.

## Quota executable selection

The new hosted quota filesystem now mounts, but direct process inspection shows that the non-root daemon has zero effective/permitted capabilities despite Kubernetes adding SYS_ADMIN to its bounding set. Select the Substrate image's byte-identical substrate-daemon-quota executable only when projectQuotas.enabled. That root-owned executable carries cap_sys_admin=ep; the default entrypoint remains plain. Preserve UID/GID65532, only SYS_ADMIN in the bounding set, the existing privilege-escalation opt-in, read-only root and absence of host mounts. Release the corrected chart as immutable 0.8.23 and deploy it with the image that supplies the quota executable; never pair that command with an older image. Final runtime evidence must prove parent/worker capability masks and actual enforced quota facts.

## Atomic storage cutover

The independent private deployment review identified an atomic rollback hazard during storage migration: a one-stage upgrade may accept writes on the new filesystem before another workload makes Helm roll back to the stale old mount. A prior scale-down is not a durable freeze because component replicas use a default expression that converts explicit zero to one, and Substrate replicas are fixed at one.

Honor an explicit zero for composed workload replicas, and expose Substrate replicas as zero or one with default one. Preserve enabled resources and PVCs while stopped. Add rendered regression checks proving the maintenance values keep both writers at zero and retain the state/workspace claims. Prepare chart 0.8.24 without rebuilding application images. The private operator can then first commit the new disk/image with both writers stopped, and enable writers in a second upgrade whose automatic rollback target already uses the same quota filesystem. Post-write rollback must retain enforced quota storage while any quota-bound workspace survives.

## Stopped cutover validation

The maintenance regression fails against the predecessor because an explicitly stopped Workspace still renders one replica. The corrected chart passes the full rollout checks: Workspace and Substrate render zero while their controllers, services, workspace claim and Substrate state claim template remain; ordinary values retain one and invalid Substrate replica counts are refused. Helm 3.19 lint, version consistency and diff checks pass. The independent read-only review found nothing and is recorded verbatim in review-result:projects-quota-cutover-pass-1.

Chart 0.8.24 introduced stopped replicas without changing application images. The final chart 0.8.25 was subsequently deployed through two successful upgrades: the first selected the new disk/image/command with both writers stopped, and the second restored writers after verifying that rollback target and absence of competing deployment.

## Nested volume initialization

Executing the rendered chart 0.8.24 volume-permissions command with production-equivalent CHOWN and FOWNER capabilities reproduces permission denied for an existing owner-private state volume and nested workspace mount. The root initializer cannot traverse a mode-0700 parent already owned by the non-root daemon. Temporarily own the state parent while initializing nested mounts, then hand the parent back last. Preserve permissions, the capability allowlist, TLS file ownership and repeatable initialization. Add execution-level checks for initial and repeated startup and publish chart 0.8.25 without rebuilding the application. The correction was independently reviewed, published and included in the completed two-stage storage deployment.

## Nested initializer validation

The execution regression runs the chart-rendered initializer with CHOWN/FOWNER only, no privilege escalation, a read-only root and private state/runtime/TLS mounts. The predecessor passes four shared-layout cases and fails all four separate-mount cases. The corrected chart passes all eight initial/repeated cases, preserving existing state and hidden workspace bytes plus directory and TLS ownership/modes. The full chart rollout checks, Helm 3.19 lint, version consistency against application 0.8.21, shell syntax and diff checks pass. All disposable fixture containers were removed. Independent read-only review found nothing and is recorded verbatim in review-result:projects-quota-init-order-pass-1. Chart 0.8.25 is published at sha256:46d746c45d6cf08ec46dfed3ac5e8fc6372631bbe0477d3f02eecc8cee7d54b0 from source 33f132d81772ac70d822e2119ac28dc5a7e75842; CI33994003285 and release33994505584 succeeded. The final published Substrate0.7.5 image passed byte/inode enforcement, current usage, isolation, destruction and identifier-reuse checks on the hosted filesystem before migration. The two deployment stages completed and all original workspace records were preserved.

## Deployed verification and remaining browser checks

Server 0.8.21 was published from source 003f038301d5448d0771342b52868dd268026f8b at sha256:659625a01169d1e89adceaac801fe165baed2351f70a1b2e61a690f5d1ec6971. Source CI33982745739 and release33983503147 succeeded. The final local application gate passed 46 frontend unit cases, 29 browser cases with 15 existing platform skips, and 67 root plus four nested Rust cases. The chart-only repairs reuse this application image and all other unaffected service images.

Earlier authenticated post-release verification confirmed repository search, project detail, branch listing and default-branch selection. For the same 18-branch repository, branch loading fell from 12048 ms to 1009 ms and selection succeeded in 1310 ms. These timings measure branch discovery and selection, not an editable workspace. The then-observed file-preparation refusal led to the separately released quota repair and verified two-stage storage deployment; workload readiness and public HTTP checks now pass.

The operator requires all further UI verification to use a headless browser. The final headless diagnostic returns AUTH_REQUIRED before project data is available, so an existing sign-in method is still required. A real file tree, read/edit/restore/close, both repository-chat and coding-Agent interactions, and end-to-end startup timing remain pending in the downstream coordination story. This source story remains active; readiness and the isolated quota proof are not substituted for those browser acceptance checks. The runtime's separate terminal execution profile remains unserved.
