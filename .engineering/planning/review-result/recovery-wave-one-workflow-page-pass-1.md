---
format: aep.planning-md/1
id: review-result:recovery-wave-one-workflow-page-pass-1
kind: review-result
status: active
title: 'Adversarial review: Workflow library page contract pass 1'
summary: The proposed page-size correction contradicted the released Workflow contract and must be reverted.
owner: wave-adversary
relations:
- reviews: story:restore-workflow-library
revision: 1
---
## Report

unit: story:restore-workflow-library
verdict: red
cases: executed 2→2, red 1
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes

- `WF-PAGE-001` — high. The released Workflow/Service SDK contract permits page limits `1..=1000`; `PageRequest::new` uses `MAX_PAGE_ROWS = 1_000`, and generated Workflow OpenAPI declares `maximum: 1000`. Invalid pagination returns HTTP 400 `invalid_page`, not 422. Therefore the previous `1000` was valid and lowering it to `100` does not establish or fix the reported refusal. Smallest correction: restore `1000`, capture the actual upstream `Problem` status/code through a generated-router or HTTP boundary test, then fix the real refusal or deployed-version skew.
- `WF-PAGE-002` — medium. Lowering the shared helper to `100` truncates list results plus detail drafts/revisions ten times earlier. Devcenter exposes only a partial-result notice and no continuation action, so entries 101–1000 become inaccessible despite being supported by the released service. Smallest correction: retain the supported `1000` until cursor pagination is implemented.

```findings
- file: crates/devcenter-http/src/lib.rs
  line: 775
  category: contract-drift
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: lowering the valid released Workflow page limit does not reproduce or fix the reported refusal
- file: crates/devcenter-http/src/lib.rs
  line: 775
  category: boundary
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: the lowered shared page limit makes Workflow rows 101 through 1000 inaccessible without cursor navigation
```
