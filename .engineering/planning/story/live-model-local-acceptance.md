---
format: aep.planning-md/1
id: story:live-model-local-acceptance
kind: story
status: active
title: Require real Claude authorization and model replies before deployment
relations:
- decomposes: epic:independent-component-delivery
- informed_by: story:local-k3d-acceptance
- informed_by: story:refresh-user-bound-model-credential
scope:
- confidence: cited
  path: crates/devcenterctl/examples/local-provider.rs
- confidence: cited
  path: crates/devcenterctl/src/local.rs
- confidence: cited
  path: crates/devcenterctl/src/local_bootstrap.rs
- confidence: cited
  path: crates/devcenterctl/src/local_build.rs
- confidence: cited
  path: docs/local-acceptance.md
- confidence: cited
  path: frontend/acceptance/history.spec.ts
- confidence: cited
  path: frontend/acceptance/model.spec.ts
- confidence: cited
  path: frontend/acceptance/setup.spec.ts
- confidence: cited
  path: frontend/acceptance/workspace.spec.ts
revision: 11
---
## Outcome

Local deployment acceptance uses the production Claude OAuth and model endpoints, normal user-owned Connector custody, and real replies across the main Agents, project and coding surfaces before a deployment is eligible for promotion. The operator explicitly rejected a mocked local model because it concealed authorization and provider failures.

## Acceptance

- Local preparation no longer redirects model traffic to the external fixture or inserts synthetic model credentials. Public HTTPS egress permits the real provider, with normal certificate verification.
- The local URL remains usable when real authorization is missing. Setup records the actual Identity session and project; acceptance returns nonzero with the local Connections URL until a user authorizes Claude and actual model attempts succeed.
- Existing real credentials survive repeated setup. Credentials and authorization codes remain in Connector custody and never enter source, chat, evidence, or command arguments.
- A fresh main Agents attempt plus project and coding attempts must complete with expected responses. Credential presence alone cannot pass. Evidence records provider configuration, immutable running images and each task outcome.
- A failed or incomplete attempt replaces the latest acceptance status; an old fixture pass cannot be mistaken for current readiness. Historical terminal pods do not count as running workloads, while pending or unready live workloads still fail.
- Targeted regressions verify live endpoint selection from both production and former fixture baselines, no synthetic model provisioning, and failure semantics. The full required source gates remain mandatory before publication.

## Scope

Cited: crates/devcenterctl/src/local.rs, local_bootstrap.rs and local_build.rs own local CLI setup, endpoint selection and evidence. frontend/acceptance/setup.spec.ts currently inserts a fake model credential, while workspace.spec.ts checks project and coding replies. Add frontend/acceptance/model.spec.ts for main Agents using the real model. Remove model emulation from crates/devcenterctl/examples/local-provider.rs. Update docs/local-acceptance.md with the real authorization and repeat-test path. No new product domain entity is introduced.

## Execution

The operator explicitly authorized this implementation. This is one bounded correction; no decomposition panel is needed for a single story. Advance draft through proposed to active before source changes, keeping live authorization and model success unproven until observed.

## Local execution evidence

The updated CLI rebuilt only the composed Connector candidate in 35.6 seconds, reused the unchanged server digest, removed the fixture server's model route, enabled real-provider HTTPS egress and rolled Agent Platform onto its production model endpoint. All application containers became ready. The prior setup's synthetic model credential was retired through the normal local BFF disconnect operation; future setup performs no model credential writes.

The new main Agents gate returned nonzero with AUTHORIZATION_REQUIRED and recorded provider_mode=live plus result=not_completed. Setup preserved its actual Identity session and project so the operator can authorize through the retained local Connections page, then rerun local test without a build or installation. No successful live model reply is claimed. A bounded invalid-code browser diagnostic exercised the local OAuth start (200), refusal (422) and consumed-flow replay (410); it does not establish valid authorization. The actual running model endpoint and image digests are retained in composition.json.

Credential-free evidence: local-evidence:devcenter-claude-20260907/local-live-up.log, local-live-oauth-diagnostic.json and local-model-fixture-retired.json. Source checks passed: frontend check (51 tests), full Rust workspace tests and clippy, composed Connector tests (4) and clippy, formatting, version consistency, chart lint, eight volume rollout checks and confidential-marker scan. Browser regression CI remains separate from live-provider acceptance.

## Independent checks after model refusal

Local 0.7 testing exposed two composition failures while model authorization was pending. The CLI now runs history and workspace checks after an independent model failure and aggregates every failed suite without marking acceptance passed. Setup remains a prerequisite. Browser-only retry obtains the current project through the authenticated API rather than requiring a setup-only project.json file in its new evidence directory. A unit regression verifies independent successes cannot hide a failed suite or retain an older pass.

## Real provider acceptance passed

The corrected Connectors 0.7 composition passed actual local k3d acceptance, followed by a second browser-only retry without a rebuild or rollout. Both runs obtained fresh nonce-checked Claude replies in main Agents, project chat and coding chat, exercised history pagination, edited and exactly restored a workspace file, executed a real PTY command and closed their owned workspaces. The running model endpoint was https://api.anthropic.com/v1. Model authorization became available during execution; no assertion is made about who completed it.

Evidence: local-evidence:devcenter-local-k3d-20260906/acceptance-1788773149796210203 and acceptance-1788773500893271555. The retry records checks.json with an empty failed list and provider_mode=live/result=pass. The setup-only project.json file is absent from the retry directory, proving the new API lookup is exercised. Targeted devcenterctl tests and clippy plus frontend check passed after the runner correction. Upstream OIDC and Git forge remain explicit fixtures. Remote promotion still requires published artifact verification and remote user-journey evidence.

## Browser readiness in deployed acceptance

Published-image local acceptance passed, but the deployed workspace test clicked Agent chat while the browser still displayed Preparing workspace files. Its separate API poll had already observed Ready; the explorer predicate incorrectly matched the Load workspace placeholder, and the loading-progress element was not yet mounted. The resulting absence of a layout POST was a test readiness failure, not evidence that the UI had finished loading.

A bounded remote diagnostic awaited the actual README entry, then observed layout initialization and Agent focus persist with HTTP 200 and the composer visible. Its owned workspace closed normally. Evidence: local-evidence:devcenter-claude-20260907/remote-focus-YK1ji5 and remote-focus-AHTpeA. Tighten frontend/acceptance/workspace.spec.ts to await the real repository file before navigation, keeping the API, persistence, editor, terminal and model assertions. Verify against the published local and remote composition. This changes acceptance timing only; no application image behavior is changed.

## Browser readiness verification

After the readiness correction, frontend check passed and the complete local run against published 0.8.35 passed again with actual Claude replies across all three surfaces, Files edit/restore, PTY execution and history checks. Evidence: local-evidence:devcenter-local-k3d-20260906/acceptance-1788776449880916246.

The corrected remote run passed Files entry, repeated navigation, editor editing/save/exact restoration, binary PTY input/output, terminal termination and normal owned workspace closure. Evidence: local-evidence:devcenter-claude-20260907/remote-readiness-retry/deployment-acceptance-JCmH9g/result.json. Coding and project replies still failed, so that run correctly records DEPLOYMENT_ACCEPTANCE_FAILED. The separate main Agents attempt also failed model_credential_unavailable. The connection owner has been asked to complete the normal deployed reconnect flow; no full remote acceptance is asserted.
