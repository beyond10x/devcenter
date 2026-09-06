---
format: aep.planning-md/1
id: review-result:files-entry-pass-1
kind: review-result
status: active
title: Files entry review identifies saved-layout persistence and close regressions
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
needs-revision

Two actionable findings prevent approval of the initially reviewed Files-entry implementation against `0657a6280e39fd2bc6de67793a07bed66c61a904`. The coordinator has accepted both findings and is preparing corrections; this report records the original judgment and does not approve those later changes.

R1 — A saved chat-only layout cannot persist focus on the new Files pane. The constructor now exposes Files locally (`frontend/src/features/workbench/devcenterWorkbenchHost.ts:85`), and hydration merges it into local panes even when the existing durable workbench contains only chat (`frontend/src/features/workbench/devcenterWorkbenchHost.ts:866`). Focusing Files does not mark that unchanged local pane as changed. Mutation flushing reconstructs the request from durable panes plus changed panes, omits Files, and substitutes chat as the focused pane (`frontend/src/features/workbench/devcenterWorkbenchHost.ts:989`, `frontend/src/features/workbench/devcenterWorkbenchHost.ts:1020`). The resulting action is focus_pane(files), while focused_pane is chat. The real BFF rejects that mismatch (`crates/devcenter-http/src/lib.rs:2907`), leaving layout persistence in error and the queued mutation unresolved. Include required local panes absent from durable state when constructing the mutation, preserving existing merge/race protections. Reproduce this from an existing chat-only layout and assert an accepted, correctly bound persisted Agent-to-Files transition.

R2 — Closing the Files placeholder restores the explorer navigation defect. AgentIDE exposes a Close Files button, but closePane protects only chat (`frontend/src/features/workbench/devcenterWorkbenchHost.ts:427`). Before a file has been opened, closing Files removes the only editor-kind pane. The pinned renderer's Workspace explorer action then only reloads the tree because it cannot find an editor pane; it does not leave Agent chat or recreate the Files view. Keep this neutral pane persistent, as chat already is, or explicitly recreate it through navigation. Cover Close Files, Agent chat, then Workspace explorer before opening any file.

The submitted browser tests pass their current cases but miss these states. Their workbench starts absent and their mutation mock accepts arbitrary pane/focus combinations (`frontend/e2e/devcenter.spec.ts:326`, `frontend/e2e/devcenter.spec.ts:832`). Consequently initialization includes Files from the outset, even in the new session-resume tests. The full retained browser run reports 32 passes and 18 skips; that is not evidence for compatibility with an older saved layout or closure of the placeholder. Use a saved-layout fixture and enforce the BFF's focus-target consistency in the mock. The existing delayed-layout test does protect local file edits from later hydration, and should remain intact.

No other actionable defect was identified in this pass. Project Files reuses the normal create/resume route, limits reuse to Ready or Preparing at the selected revision, and retains the disabled-feature read-only preview (`frontend/src/features/projects/ProjectsView.vue:82`, `frontend/src/features/projects/ProjectsView.vue:210`). Explicit editor entry is consumed when the host is created, and late layout adoption retains the existing user-touched-focus guard. Pre-ready file, mutation, and background-service calls remain refused or deferred; this change does not grant pre-ready workbench effects. Return actions cover Refused, Closing, and Closed session states. Query-only changes on an already mounted session are not a new navigation trigger; the preference is an entry-time choice.

The placeholder's semantic shape is supported: the pinned renderer Pane type permits an editor without a file path, and its projection can be empty. The BFF validates optional path/cursor fields without requiring an Editor path. AgentIDE's PaneSnapshot definition has no kind-specific required path, and the generated Connector surface input accepts the existing pane list representation. This review does not mistake the old Devcenter client package versions for deployed service versions, and makes no claim that terminal availability or model credentials are repaired.

The original runtime/test/version diff comprises `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `frontend/e2e/devcenter.spec.ts`, `frontend/package.json`, `frontend/src/features/projects/ProjectsView.vue`, `frontend/src/features/workbench/HostedWorkspaceView.vue`, `frontend/src/features/workbench/devcenterWorkbenchHost.ts`, and `openapi.json`. Its SHA256 is `37a3a41a6ea471d71a5bf5bda197bca99f0b89eb840f8ed0ed44f57031f751e2`; planning records are excluded. Whitespace checks pass. The reviewer performed source, schema, and evidence reads only, did not duplicate running gates, and made no live session, credential, repository, planning, Git, or provider mutation. This private report is the sole write.

```findings
[
  {
    "file": "frontend/src/features/workbench/devcenterWorkbenchHost.ts",
    "line": 85,
    "category": "correctness",
    "severity": "blocker",
    "message": "R1: The new Files pane exists locally but is absent from an older chat-only durable workbench. Focusing it produces changed=[]; flushing reconstructs only durable chat, substitutes focused_pane=chat, and sends action focus_pane(files), which the BFF rejects. Include absent-durable local panes in the persisted mutation while preserving merge protections, and add a saved chat-only layout regression that validates the actual focus target and accepted persisted response."
  },
  {
    "file": "frontend/src/features/workbench/devcenterWorkbenchHost.ts",
    "line": 429,
    "category": "correctness",
    "severity": "warning",
    "message": "R2: closePane protects only chat, so Close Files removes the sole editor-kind pane before any file is open. Workspace explorer then reloads the tree without leaving Agent chat, restoring the reported navigation defect. Preserve or recreate the neutral Files pane and cover Close Files -> Agent chat -> Workspace explorer before opening a file."
  }
]
```
