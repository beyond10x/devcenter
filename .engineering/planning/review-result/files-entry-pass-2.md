---
format: aep.planning-md/1
id: review-result:files-entry-pass-2
kind: review-result
status: active
title: Files entry correction approved after saved-layout regression fixes
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

No remaining actionable findings in the final Files-entry implementation against `0657a6280e39fd2bc6de67793a07bed66c61a904`. Both findings from the initial review are resolved. This is a separate second review; the initial needs-revision report remains unchanged.

R1 is resolved at `frontend/src/features/workbench/devcenterWorkbenchHost.ts:989`: a pending mutation now includes a local pane if its contents changed or its identifier is absent from the durable workbench. For an older chat-only layout, this includes Files even though it was already exposed locally. The existing flush logic merges that pane by identifier before selecting focus, so a Files-focus action now carries Files in panes and focused_pane=files. Existing removal sets, serialized mutation processing, idempotency keys, monotonic durable adoption, and the user-touched-focus guard remain intact. Including local panes while durable hydration is still pending also retains the existing immediate-edit behavior without requiring early layout completion.

R2 is resolved at `frontend/src/features/workbench/devcenterWorkbenchHost.ts:429`: closing chat or Files returns before modifying panes, recording closure, or queuing a backend mutation. The neutral Files view therefore remains available for the renderer's editor navigation even before any real file is open. File-backed editor panes retain their existing close behavior.

The new regression starts both Ready and Preparing resume cases with an existing chat-only saved layout (`frontend/e2e/devcenter.spec.ts:332`). The mock now rejects a focus action if its target differs from focused_pane or is absent from panes, matching the relevant BFF checks (`frontend/e2e/devcenter.spec.ts:859`, `crates/devcenter-http/src/lib.rs:2872`, `crates/devcenter-http/src/lib.rs:2907`). The tests wait for an actual successful persisted focus response, exercise Agent-to-Files, then Close Files-to-Agent-to-Files, open a real editor, and verify that resume creates no new session (`frontend/e2e/devcenter.spec.ts:1938`). The deciding predecessor run fails both resume cases with 422 instead of the required 200; the corrected full production browser suite passes. These assertions directly cover the two initial findings rather than merely accepting an optimistic local projection.

The rest of the final flow remains consistent with the requested behavior. Project Files enters the normal create/resume route, reuses only a Ready or Preparing session at the selected revision, and preserves the disabled-feature read-only preview. Entry with pane=editor selects a restored file-backed editor when available, otherwise Files; later layout adoption cannot override user-touched focus. Preparation still defers tree and ancillary calls until Ready, and existing authority/readiness guards continue to govern effects. Refused, Closing, and Closed sessions offer a project return. The earlier delayed-layout/edit regression remains in the passing suite. Placeholder Editor panes without a file path remain compatible with the inspected renderer types, BFF checks, and existing AgentIDE surface representation.

Reviewed final frontend evidence passes formatting, lint, generated-type checks, type checking, 46 unit tests, and production build. The production browser suite passes 32 cases with 18 intentional skips. The retained leak check is clean. The coordinator reports the root and composed Rust gates and chart checks passed; those broader commands were not repeated by this reviewer. Whitespace checks pass. Browser APIs are mocked, including the relevant focus contract; these results do not constitute authenticated deployed acceptance or prove every AgentIDE backend constraint.

Independent parsed comparisons confirm consistent server candidate 0.8.30 across Cargo, frontend, and OpenAPI metadata, with only eight local root package versions changing in Cargo.lock. The frontend dependency lock, independent Connector composition, and chart are unchanged. No server authority, credential custody, terminal, or model-credential correction is part of this diff. Terminal availability and model credentials remain unresolved outside this review.

The exact final runtime/test/version diff comprises `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `frontend/e2e/devcenter.spec.ts`, `frontend/package.json`, `frontend/src/features/projects/ProjectsView.vue`, `frontend/src/features/workbench/HostedWorkspaceView.vue`, `frontend/src/features/workbench/devcenterWorkbenchHost.ts`, and `openapi.json`. Excluding planning records, its SHA256 is `5195c9615112574d51cc64184782707d80ad3431b6013ec48b26ea8e828ad887`. Recommend accepting this final implementation for normal source CI and release processing. Publication, deployment, and authenticated Files-entry acceptance remain separate evidence. The only reviewer write is this private report; no repository, planning, Git, provider, browser credential, or live session mutation occurred.

```findings
[]
```
