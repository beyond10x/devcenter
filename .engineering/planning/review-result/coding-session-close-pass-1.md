---
format: aep.planning-md/1
id: review-result:coding-session-close-pass-1
kind: review-result
status: active
title: Independent coding-session coordination close review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

No actionable findings in the corrected implementation reviewed against `76b89f0ba76f0373bf7bb78cbbaf2eb234dce273`.

The close-specific check accepts an omitted Workspace manifest only for Closing, Closed, Unknown, or Refused (`crates/devcenter-http/src/lib.rs:2414`). It still requires exact string bindings for both session identifiers, project, and source revision (`crates/devcenter-http/src/lib.rs:2387`). When Workspace supplies a manifest, equality remains mandatory. This accommodates the existing cleanup response contract without accepting a different session or source.

Only the close handler uses the exception (`crates/devcenter-http/src/lib.rs:3237`). Ordinary coordination creation, reading, and coding-turn version checks retain the strict manifest comparison (`crates/devcenter-http/src/lib.rs:2447`, `crates/devcenter-http/src/lib.rs:2497`, `crates/devcenter-http/src/lib.rs:2517`, `crates/devcenter-http/src/lib.rs:2965`). Close still authenticates a mutation, asks Workspace to close the owned session, queries coordination under the same identity, and uses the existing expected version and deterministic idempotency key (`crates/devcenter-http/src/lib.rs:3216`). No credential custody, tenant derivation, route admission, or Connector authority changes are introduced. An already closed coordination snapshot remains idempotent. Invalid binding, or an active snapshot without a version, still fails without submitting a close intent.

The added test covers all four allowed cleanup states, rejects mismatched and missing values for each immutable binding, preserves strict checks in ordinary paths, rejects digest omission against an existing digest in Preparing and Ready, and checks equal and unequal supplied manifests (`crates/devcenter-http/src/lib.rs:6363`). Both Active and Closed coordination snapshots are covered. This is a focused helper regression, not a complete HTTP/Connector close integration test or deployed acceptance result.

Reviewed root Rust evidence passes all 68 cases, including the new regression. The initial new-test numeric-literal lint was corrected without changing values; the final Clippy log passes with warnings denied. The earlier missing frontend build prerequisite was resolved before the successful root test run. The reviewer inspected evidence and did not rerun builds. Whitespace checks pass. `Cargo.toml`, `Cargo.lock`, `frontend/package.json`, and `openapi.json` consistently advance the root release to 0.8.28; only local root package versions change in the lockfile, and the independent Connector composition remains unchanged. The older Workspace client dependency is not evidence of the deployed service version and does not change in this patch.

The SHA256 of the reviewed diff for `crates/devcenter-http/src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, `frontend/package.json`, and `openapi.json` is `b23791b61f996b8164a5278077b2094d7f330d26d0877787c2b0ca9b4c76e873`. Recommend accepting this source fix. Remaining release gates and an authenticated normal close after deployment are separate evidence. Planning records and publication were outside this read-only review.

```findings
[]
```
