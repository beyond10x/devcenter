---
format: aep.planning-md/1
id: story:local-live-integrations
kind: story
status: active
title: Test real integrations in the retained local deployment
relations:
- informed_by: story:local-k3d-acceptance
- informed_by: story:agent-management-controls
scope:
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: crates/devcenter-connectors/Cargo.lock
- confidence: cited
  path: crates/devcenter-connectors/Cargo.toml
- confidence: cited
  path: crates/devcenterctl/src/local.rs
- confidence: cited
  path: crates/devcenterctl/src/local_bootstrap.rs
- confidence: cited
  path: crates/devcenterctl/src/local_build.rs
- confidence: cited
  path: crates/devcenterctl/src/local_integrations.rs
- confidence: cited
  path: docs/local-acceptance.md
- confidence: cited
  path: frontend/acceptance
- confidence: cited
  path: frontend/e2e/devcenter.spec.ts
- confidence: cited
  path: frontend/src/composables/useConnectSessionPolling.ts
- confidence: cited
  path: frontend/src/features/connections
- confidence: cited
  path: frontend/src/features/connectors
- confidence: cited
  path: frontend/tests/connect-session-polling.test.ts
- confidence: cited
  path: frontend/tests/connections.test.ts
revision: 22
---
## Outcome

The retained local deployment exercises real activated provider integrations before publication. Private deployment configuration supplies the repository forge, collaboration provider and observability origin; generic source owns no organization-specific identifiers.

## Acceptance

- Explicit live mode preserves the private provider configuration and normal Identity, Connector custody and grants. Fixture mode is recorded separately from the always-real model mode.
- Generic setup uses declared administrative requirements or owner-scoped Connect Sessions. Provider endpoint configuration alone never manufactures a credential or connection.
- Credential values stay in normal custody or owner-only input files, never source, command arguments, environment variables, logs or evidence.
- Actual freshly described read operations succeed for every configured provider, and the real repository materializes with working Files and terminal before complete acceptance.
- Only changed components are built; normal iterations retain the cluster, databases, model connection, CA, quota state and caches.

## Implementation

The local Rust CLI retains live/fixture selection, clears inherited fixture provisioning on mode changes, supports a local Agent Platform source build, and invalidates acceptance receipts on prepare/apply/test. A live test refuses a fixture-derived last project. Required provider reads derive from composed private configuration and use fresh admitted operation descriptions.

Connectors endpoint and form candidate 129aece1ef6ae3717229cf1505a40106cdee92c1 is composed and deployed locally. The hosted catalog form renders the selected profile's credential label, help and documentation, including the observability service-account-token versus SSO-password distinction. Native administrative requirements remain distinct from generic catalog principal-owned credentials.

## Verification

The required Devcenter source gates pass, including frontend checks and browser regressions, both root and standalone locked Rust workspaces, chart, version, leak and diff checks. Upstream Connector source gates pass for the exact consumed candidate. Actual agent and capability-profile lifecycle checks previously passed against this retained composition.

The latest local observability acceptance passes: deployed BFF pending-session status reads return200, the saved connection displays Connected after browser reload, and a freshly described real provider read succeeds with an audit reference. Detailed source and deployed evidence is recorded below. No credential resubmission was needed after the owner saved the original connection.

Real Git and collaboration-provider authorization remain pending. The retained Git connection and project are fixture-derived and cannot satisfy live repository materialization or PTY acceptance. The latest real conversation recall remains refused with category reasoning_extraction despite the typed-history correction. These remain explicit failures or missing authorization, rather than being replaced by a preceding successful run. The story remains active; no release or complete live acceptance is claimed.

## Programmatic setup follow-up

The generic hosted token CLI/client candidate reuses the existing owner-scoped Connect Session. It accepts an owner-only token file, validates the exact completion destination, submits once without redirects or automatic retry, and confirms owner-scoped status and connection description. The CLI checks its catalog declaration before reading credentials or opening Identity state. Its supported scope is single-secret catalog token profiles; OAuth consent and native multi-field acquisition keep their normal routes. The reusable protocol client remains independent of catalog implementation and supports normally issued short-lived Identity authority without a desktop keyring.

The candidate passed focused client and CLI boundary tests and all-target clippy. Native CA support was verified with TLS tests and then through actual normal Identity login against the retained local deployment. A fresh headless browser completed upstream login and the loopback callback; the CLI stored its session through the normal OS keyring and selected the local deployment in an isolated XDG state directory. A subsequent owner-scoped hosted connection list succeeded. The ordinary CLI metadata still selects the existing non-local deployment. The first browser helper attempt failed before browser startup because its TMPDIR was not set; after correcting that environment prerequisite, the normal login passed. Failed evidence is retained.

The Grafana operation search still returned no admitted operations, and no provider token has been supplied. Thus actual token acquisition and external reads remain unverified. The candidate is published at acf8ef8c01e6a794860babd5d6df15a2d71b8014. Its full repository gate passed all twelve workspaces, both runtime feature configurations, catalog and documentation checks, and exact ESS projection. Client, CLI and console all-target clippy passed. The tested CLI binary matches the final implementation; only evidence updates followed its build. This is a client-only candidate compatible with the deployed endpoint/form runtime; no service image rebuild or shared deployment was performed.

## Token refusal and private egress correction

The operator supplied a service account token but both retained and freshly opened catalog forms returned a generic refusal. Source inspection found verification discards bounded failure categories and the form also hides expiry; that upstream diagnostic correction is delegated to the Connector owner.

The composed Kubernetes NetworkPolicy allows public HTTPS while excluding private IPv4 ranges. The private observability target resolves inside an excluded range. The local bootstrap also replaces all private baseline extraEgress rules even in live mode, removing required private endpoint access. Correct live-mode preparation to preserve explicitly declared private egress and append the local ingress rule idempotently; fixture behavior remains scoped to its fixture. The downstream deployment supplies exact destination CIDRs and ports. Verify a pod governed by the same egress policy before and after the official CLI apply, and retain provider acquisition/read as unverified until an actual token attempt succeeds.

## Credential custody collision

The live owner retry reached provider verification successfully, then returned a generic503 during credential persistence. Read-only deployed metadata inspection confirmed a shared prepared-secret retirement watermark ahead of the catalog and another provider's independent generation counters. The catalog had no pending or completed connection. This is an upstream cross-provider transaction-domain defect; do not alter deployed counters manually or ask the owner to replay the credential until the runtime correction and recovery regression pass.

The owning Connectors implementation must preserve pending recovery and retirement fences, cover interleaved native and catalog acquisitions, and expose only closed completion-stage/error classifications. Consume the published correction through the composed runtime and prove the complete custody path before another owner retry. Neither the network health probe nor a corrected form establishes provider acceptance. Retain the existing live private egress policy and component image selections during composition.

## Credential custody correction verification

The shared custody correction is published upstream at df875e22dc9eb95b009d7626966c11a3d587b4f4 and consumed by the standalone composed Connector manifest and lock. Only that Connector revision changed in the lock; the separately selected SDK, Eventlog, Agent Platform and other source pins remain intact.

Upstream verification passed all twelve source workspaces, both runtime feature configurations, all-workspace formatting, root/runtime all-target clippy with denied warnings, and final catalog/docs/ESS checks. Five composed journal regressions exercise the actual PreparedVaultStore with controlled state/value adapters and provider HTTP. Eight shared-helper regressions include FileStore recovery; eighteen actual Chromium cases exercise the source completion script against controlled responses. These are not live provider acceptance. A separate test-owned live Secrets sentinel roundtrip passed stage/read/publish/read/delete and confirmed both addresses absent; the diagnostic pod was removed.

The consuming standalone workspace passed locked clippy, tests and formatting. Documentation formatting, version consistency, leak checks and diff checks pass; the preceding root workspace checks remain applicable because its implementation and dependencies did not change. The actual Connector image was built, pushed to the retained local registry, and applied through the official local CLI with the matching private values and lock. Only the Connector image changed and the previously corrected private egress declarations were preserved.

The running image digest matches the built and pushed candidate. All twelve other running pods retained their identities, image digests and restart counts; the owner-scoped connection list is unchanged. The new Connector needed two startup retries with a database connection error, then became ready and stayed at two restarts through verification. The node reports Ready with all pressure conditions false. A first image-verification helper assertion confused the runtime configuration digest with a manifest reference; the corrected check independently matches the pod specification, runtime imageID and local image configuration digest.

An actual headless browser verified the newly served Grafana token form, SSO guidance, required data-source permission and five-minute deadline. Advancing the browser clock cleared and disabled the field and submit button without any POST. No owner credential was accessed or replayed. Real Grafana credential persistence and a freshly admitted provider read still require a new owner submission through the deployed form. Other live provider reads and the previously recorded real conversation refusal remain incomplete; no release or shared deployment is claimed.

## Saved connection polling and invocation follow-up

The owner completed the real observability token form on the custody-fixed image. The owner-scoped connection is now callable and visible through the BFF, proving persistence. The catalog UI nevertheless remained Waiting for provider and reported service unavailable. A fresh tokenless session reproduces BFF status HTTP503: pending hosted status deliberately omits the completion capability, but the protocol validator incorrectly requires a completion locator on both creation and status. Separate the creation and polling rules upstream while retaining malformed-locator and terminal-state validation; consume that protocol correction in both BFF client and composed runtime.

A real discovered and freshly described read also failed with stale_authority, including describe and invoke issued immediately in sequence. Hosted catalog creates a fresh delegated backend per request and loses its description leases. The upstream owner is preserving bounded, authority-bound delegate continuity while rechecking current owner-filtered connection and binding configuration. Regressions must use public hosted request dispatch rather than a retained internal delegate that bypasses the actual bug.

The UI must keep polling through the actual session deadline, show failed status checks as unconfirmed instead of failed authorization, and offer a status-only recovery action without resubmitting credentials. Completed sessions refresh saved connections. Verify pending polling through the real BFF, existing saved connection visibility after reload, and an admitted provider read against the same populated retained deployment. The owner must not submit the already-saved token again.

## Polling and invocation source verification

The combined upstream correction is published at ff9cc7a5b721874e3b21e3ce22bbdd4e099a35d6. Both root client/protocol dependencies and the standalone composed runtime select that exact revision. Targeted Cargo resolution preserved every package version and source repository; the root also enables the upstream client's native certificate support already used by the normal local CLI.

The shared frontend poller retains the issued authorization session on status errors, offers a status-only retry, observes the real deadline, and rejects stale or disposed callbacks. Catalog reload now derives Connected from the current saved owner connection. The frontend gate passes 57 unit tests and 40 browser tests across desktop and mobile, including a first status503 followed by successful recovery using exactly one authorization creation. Eighteen existing conditional browser skips remain. The root and standalone Rust formatting, locked all-target clippy and tests pass (92 tests across19 result groups); version, chart lint/rollout, leak and diff checks pass.

Source checks are separate from the pending image and provider acceptance. Build only the server and Connector, retain the populated state and matching private baseline, then prove actual BFF pending status200, saved connection visibility after reload, and a freshly described live provider read. No new credential submission is needed or authorized by these checks.

## Deployed polling and real provider verification

The exact server and Connector candidates were built, pushed to the retained local registry, and applied through the official CLI. The running server digest is sha256:18836b62a8123bf4efef01de24b2c706785b0d9db01202b10f4bf992857ce67c; the Connector digest is sha256:d1b17a5ef97e3b5eaac576dcf8e237543570f730741726addd8ec03183941f40. Pod specifications, runtime image IDs and local configuration digests match. All eleven unselected running pods retain their identities, images and restart counts; owner-scoped connections are unchanged. Private egress and the matching baseline were retained. The server started without a restart; the Connector required one database-related startup restart, then became ready. The node reports Ready with all pressure conditions false; the restart evidence remains recorded as a reliability issue.

The deployed headless browser test passed three actual pending-session BFF status reads, all HTTP200. Reload showed the saved observability connection as callable, and its catalog page displayed Connected. It opened only a tokenless pending attempt, made zero credential submissions, and closed the popup and browser. The issued pending attempt expires naturally; no saved credential was changed.

The official live-integration acceptance passed both tests. Actual operation discovery, fresh description and invocation all returned HTTP200 for grafana-datasources-list, with a nonempty Connector audit reference. This read uses the owner's saved credential against the private real provider. It is not a fixture or credential-presence assertion. Private evidence includes grafana-operational-ui-691279696281590/result.json, grafana-live-acceptance-1788811577096925404/integration-grafana.json and grafana-operational-composition-proof.json; no provider bodies or credential values are published.

This resolves the saved-connection polling and observability-read failures in the local candidate. Real Git and collaboration-provider authorization, real repository materialization and PTY, and the previously refused real conversation recall remain incomplete. The owning story stays active. No release or shared-deployment acceptance is claimed.
