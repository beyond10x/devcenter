---
format: aep.planning-md/1
id: review-result:recovery-wave-one-refresh-gitlab-authority-pass-1-normalized
kind: review-result
status: active
title: 'Adversarial review: generic connection recovery pass 1'
summary: Two high-severity authority-selection and stuck-session defects require correction.
owner: wave-adversary
relations:
- reviews: story:refresh-gitlab-authority
revision: 1
---
## Report

unit: story:refresh-gitlab-authority
verdict: red
cases: executed 2→4, red 2
origin: introduced 2, pre-existing 0, undecided 0
wrote-outside-worktree: none
needs-coordinator: yes

- `CONN-ADV-001` — high. An app-level callable GitLab grant is presented as the user’s callable connection. “Replace authorization” then submits `gitlab.application` with the deployment grant’s label instead of creating `gitlab.oauth` personal authority. Smallest correction: when a person profile exists, select only matching-profile or `actor: user` connections; never fall back to app authority.
- `CONN-ADV-002` — high. A failed connect-session status request leaves the cached session `pending`, permanently disabling retry and showing “Waiting…”. The same happens when polling reaches its 60-attempt limit. Smallest correction: clear or locally fail the pending session on polling failure/exhaustion so recovery becomes actionable again.

Released protocol boundary: generic `ConnectionRequest` has no revocation operation, and connect-session creation carries no target `connection_ref`. Provider implementations may replace authority by owner/profile, but stable-reference replacement is not guaranteed by the wire contract.

```findings
- file: frontend/src/features/connections/ConnectionsView.vue
  line: 165
  category: boundary
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: a curated person connection falls back to app authority and starts replacement with the deployment grant profile and label
- file: frontend/src/features/connections/ConnectionsView.vue
  line: 203
  category: concurrency
  severity: blocker
  verdict: CONFIRMED
  origin: introduced
  message: failed or exhausted connect-session polling leaves the cached session pending and permanently disables retry
```
