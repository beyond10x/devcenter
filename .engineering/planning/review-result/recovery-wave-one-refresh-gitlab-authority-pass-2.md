---
format: aep.planning-md/1
id: review-result:recovery-wave-one-refresh-gitlab-authority-pass-2
kind: review-result
status: active
title: 'Adversarial review: generic connection recovery pass 2'
summary: The original corrections hold; one introduced stale-state defect and two pre-existing UX gaps remain.
owner: wave-adversary
relations:
- reviews: story:refresh-gitlab-authority
revision: 1
---
## Report

unit: story:refresh-gitlab-authority
verdict: red
cases: executed 4→9, red 3
origin: introduced 1, pre-existing 2, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes

The two requested corrections hold: person authority wins over application authority, and poll failures/exhaustion affect only the matching pending session.

- `CONN-ADV2-001` — medium, pre-existing. One global `curatedStarting` string creates a race between concurrent provider starts. When GitLab finishes first, Slack becomes enabled while its own request is still pending. Use provider-keyed starting state and clear only the completing provider.
- `CONN-ADV2-002` — high, pre-existing. A blocked popup leaves the user with “Authorization pending” but no link to the retained `browser_completion_url`. Always render a safe continuation link for pending browser sessions.
- `CONN-ADV2-003` — medium, introduced. Refresh can observe callable authority while preserving a stale failed-session message, producing contradictory “Authorization failed” and “Callable” states. Reconcile terminal session state after a successful authority refresh.

```findings
- file: frontend/src/features/connections/ConnectionsView.vue
  line: 234
  category: concurrency
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: a single curatedStarting provider ref is cleared by whichever concurrent request finishes first, re-enabling another provider whose own start request remains in flight
- file: frontend/src/features/connections/ConnectionsView.vue
  line: 243
  category: recoverability
  severity: blocker
  verdict: CONFIRMED
  origin: pre-existing
  message: the browser completion URL is only passed to window.open and is never rendered, so popup blocking strands the pending authorization flow
- file: frontend/src/features/connections/ConnectionsView.vue
  line: 223
  category: state-reconciliation
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: terminal session failure remains visible after refresh observes callable authority, yielding contradictory current state and historical error
```
