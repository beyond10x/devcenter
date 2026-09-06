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
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: Dockerfile
- confidence: cited
  path: Dockerfile.ess
- confidence: cited
  path: README.md
- confidence: cited
  path: ci/check-chart-rollouts.sh
- confidence: inferred
  path: ci/check-substrate-volume-permissions.sh
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenter-http/src/lib.rs
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
  path: ess/build.yaml
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/package.json
- confidence: cited
  path: frontend/src/api/client.ts
- confidence: cited
  path: frontend/src/features/projects/ProjectsView.vue
- confidence: cited
  path: generated/ess/build.json
- confidence: cited
  path: openapi.json
revision: 35
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

## Smart HTTP authentication repair

The first new hosted coding session after the quota cutover passed quota admission and failed at Git fetch. A read-only comparison against the same provider and existing credential returned 200 for REST API Bearer authentication, 401 for Git discovery with Bearer, and 200 with a protocol-v2 advertisement for Git discovery using GitLab's documented HTTP Basic username oauth2 and the token as password. The released Git broker incorrectly reuses REST bearer_headers for Smart HTTP.

Consume the independently reviewed Connectors story:git-http-oauth-authentication source revision in the composed runtime manifest and lockfile. Publish only the Connectors image at a fresh immutable artifact version, preserving unrelated release units and the verified chart and quota filesystem. Update the downstream private deployment's exact image version/digest through its existing lock and CI. The upstream fixture must require Git-specific Basic authentication and complete a real protocol-v2 fetch while REST continues to require Bearer; credentials must remain header-only and absent from responses, URLs, and persisted state.

Source gates, an independent review and deployed image verification precede the final authenticated headless checks. The existing sign-in requirement is still pending. A separate real project Agent request is admitted and starts, then fails with model_credential_unavailable; that credential-resolution path must also be verified before claiming Agent acceptance. No database migration belongs to this runtime repair.

## Reviewed source consumption

The composed Connectors manifest and lock consume the reviewed source revision fe3541a6d866e84855dfdc19ec4d22a7e779b1e5, proposed in Connectors PR15. The deciding real-Git regression and all 40 package cases, formatting, Clippy and twelve-workspace offline metadata pass; the independent source review found no blocking findings. Full source CI34002384974 is running.

The published SDK still depends on the preceding Connectors protocol/service revision. Keep all shared nominal types on the repaired revision through workspace-root Git patches for those two contracts. Cargo requires a different source URL for the patch, so the direct runtime/contracts and overrides use the repository's SSH URL, covered by the existing delivery transport authentication. Locked metadata resolves exactly 32 Connectors packages from one source revision, with no duplicate package names or retained predecessor source. Unrelated SDK and generated-service pins stay at their released revisions.

Prepare the composed Connectors artifact 0.8.26. The release-unit classifier selects Connectors only. Version consistency and nested formatting pass. The full Devcenter gate and affected OCI build, independent consumer review, source merge, immutable publication and downstream rollout remain required; authenticated headless acceptance is still pending the existing sign-in requirement.

## Image-build transport correction

The first independent consumer review found that host Cargo authentication supports both transports but the isolated Connectors Docker stage only rewrites HTTPS dependencies. The new shared-contract source identity therefore needs the SSH prefix mapped to the same existing token-authenticated HTTPS endpoint inside that stage. Add that mapping to the authoritative ESS Connectors build command and to the maintained Dockerfile's corresponding stage. Both mappings are removed by the existing unset-all cleanup; no credential or SSH key is added to the source or image.

ESS 0.9.2 validates the existing specification and deterministically regenerates the compiled build IR and Dockerfile projection. The existing projection check passes before and after the edit; Bake and graph are unchanged. Version consistency and diff checks pass. The shared build files conservatively broaden CI impact to the image units; final publication explicitly selects only Connectors and retains the prior successful artifacts for the other units. A second independent review and the real CI OCI build must verify this remedy before publication. The first review and its finding are retained in the evidence archive; its initial unsupported findings shape was refused by AEP and a schema-admissible report has been requested from the critic.

## Deployed Git HTTP authentication repair

Devcenter PR50 merged source a15597c0d08527a553afe9562c8b3eaa255b2f01 after CI34003341500 passed the full repository gate and affected OCI builds. The independent consumer's second pass found no remaining blocking findings. Publication 0.8.26 completed successfully in CI34004333609, building both supported architectures, signing the Connectors image and validating the exact composed candidate. The immutable Connectors digest is sha256:d499c64f3a30bc2a86d3ee2b5bc887a6bbd2fb1433cd1d1f97d1b85ef7232a0a. Chart 0.8.25, server 0.8.21 and deployment CLI 0.8.18 were reused.

The downstream deployment changed only the Connectors version and digest in its three lock/value/CI inputs. Local rendering matched all eleven workload images to the lock. Its validation, atomic deployment and verification jobs all succeeded. Direct post-deployment observation confirmed that the ready Connectors pod reports the published immutable image ID with zero restarts. Other service pins and the verified quota storage remain unchanged; this authentication repair created no additional cloud resources.

The exact released Substrate 0.7.5 Git client also passed the repaired Connectors fixture, checking the admitted commit, fifty shallow-history entries, no tags, no transient authority in stored Git configuration and a spent broker session. This is a real client compatibility fixture with synthetic custody, not a hosted user session.

Post-deployment headless Chromium rendered the public sign-in page with HTTP 200, no JavaScript errors and no failed application assets. Its anonymous session request correctly returned HTTP 401. No usable authenticated browser state was supplied, so file-tree, read/edit/restore/close, both Agent replies and end-to-end workspace startup timing remain unverified. The observed Agent credential failure also remains unresolved. The story stays active. Task-local build outputs and diagnostic pods were removed, evidence retained privately, and managed worktree retirement follows publication of this record.

## Authenticated discovery follow-up

The operator authorized reusing the live browser's Devcenter session for headless acceptance. The copied session authenticated successfully: project details, branch and repository tree returned HTTP 200, and the Files tab rendered the repository entries. A new coding-session creation reached the workbench route in about 1.1 seconds, then was refused at materialization after about 8.5 seconds. Substrate recorded workspace.git-fetch-failed. A normal Identity-derived source-broker request isolated HTTP 502 to its first v2 discovery exchange.

The provider's credentialed Smart HTTP response is HTTP 200 with the expected Git advertisement content type, but prefixes version 2 with the exact upload-pack service-announcement packet and a flush. Connectors' current v2 parser rejects that optional framing. Its existing real-Git fixture omits the preamble, explaining why earlier controlled compatibility passed. Connectors story:git-v2-smart-http-preamble owns the bounded parser and fixture repair. This story consumes the reviewed source and publishes only the changed Connectors image, then repeats the authenticated browser acceptance. Readiness and the previous OAuth repair do not close the outstanding source-file defect.

The temporary browser session remains outside all repository trees and is removed after the authorized checks. No database migration or new cloud resource is required by this follow-up.

## Reviewed Git v2 framing consumption

The composed Connectors 0.8.27 candidate consumes source 53ba51fb744e223e220523ff49f313e1d23d8673 from Connectors PR16. The real HTTP fixture failed with HTTP 502 under the original parser and all 42 GitLab tests pass with the exact optional preamble correction. Independent review found no actionable defects. Full source gate CI34020120274 is running; it must pass before merge.

Locked metadata resolves all 32 Connectors packages to that one reviewed source, with no duplicate package names. Nested formatting, release version consistency and diff checks pass. Release impact is Connectors only. The complete Devcenter gate and affected OCI build precede publication; the downstream private deployment continues to be coordinated by this artifact rather than an ungoverned private planning file.

Authenticated project chat independently reproduced model_credential_unavailable. A fresh, one-use normal subscription lease was created successfully, but immediate redemption returned HTTP 400 subscription-oauth-refused. This identifies OAuth refresh or refreshed-record validation as the failure boundary. The provider connection still reports Connected because its status checks stored-record presence. Normal Claude reconnection has been requested from the operator; this source-framing change does not repair or claim Agent credential acceptance.

## Deployed framing repair and ready-session recovery

Connectors source CI34020120274 and Devcenter gate/image CI34020408981 succeeded. PR51 merged source76b89f0ba76f0373bf7bb78cbbaf2eb234dce273. Publication0.8.27 succeeded in CI34021522883 and signed the Connectors index sha256:d6a7e8621ceb223f0e562b18f1d58800bcf0f30a337b7d34ba351b3b25a29e79. The downstream deployment validation, deployment and verification jobs passed. Direct running-image verification matches that exact digest, with only Connectors changed among the eleven locked workload images.

Authenticated headless creation now reaches ready, proving live Git checkout interoperability. The next file/terminal metadata requests return not found. Exact read-only durable operation evidence proves Workspace's own cleanup operation destroys the successfully created materialization immediately after publication. Workspace story:preserve-ready-materialization owns the delayed preparing-read recovery correction and next independent runtime release. Browser file acceptance remains pending that repair.

A separate close-response defect exists in this BFF: Workspace hides its manifest hash outside ready, but the close handler compares that absent hash with AgentIDE's retained digest and refuses coordination closure. Correct only the close-specific comparison: retain exact session, Workspace-session, project and source identity checks, and validate any manifest supplied. Ordinary ready-session coordination retains its strict manifest comparison. Verify already-closed/retry paths and reject mismatched session/project/source bindings. No user authority or general mutation check is relaxed.

Authenticated project Agent still fails model_credential_unavailable, with normal one-use credential redemption returning subscription-oauth-refused. The requested normal Claude reconnection remains outstanding. Neither the deployed framing fix nor the remaining workspace fixes claim model-credential acceptance.

## Coordination close verification

Independent source review found no actionable defects in the close-specific correction and is preserved in review-result:coding-session-close-pass-1. The pinned Node22.23.1 and pnpm11.25.0 frontend install/build pass. All68 root Rust tests pass, including the new cleanup-state, immutable-binding and manifest regression. Final workspace Clippy with warnings denied, formatting, release version consistency and whitespace checks pass. A numeric-separator lint in the new fixture was corrected without changing values.

Prepare server0.8.28; only local root package versions advance. The next publication selects the server explicitly, retaining the existing independently published CLI, chart and Connectors image. Workspace0.2.21 publishes independently from its owner repository; the downstream deployment will consume both immutable runtime digests in one ordinary deployment. Full Devcenter CI and authenticated read/edit/save/restore/close remain required before product completion.
