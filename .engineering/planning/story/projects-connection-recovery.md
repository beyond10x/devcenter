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
- confidence: cited
  path: deploy/charts/devcenter/Chart.yaml
- confidence: cited
  path: deploy/charts/devcenter/templates/substrate.yaml
- confidence: cited
  path: deploy/charts/devcenter/values.schema.json
- confidence: cited
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
revision: 13
---
## Outcome

Restore the Projects repository listing when current GitLab authority admits no usable connection, and provide a visible connection recovery action without misreporting an engineering-plan failure. O1 authority remains enforced and O4 repository navigation becomes operable.

## Evidence and scope

The authenticated repository endpoint returned 502 while session and agent requests succeeded. In the released implementation, GitLab Describe returns NotFound when no connection supports gitlab-project-list; Workspace maps that refusal to 502; the BFF uses a generic Workspace refusal code whose browser text incorrectly names an engineering plan. This code path is verified; whether it is the exact hosted cause remains under investigation.

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
