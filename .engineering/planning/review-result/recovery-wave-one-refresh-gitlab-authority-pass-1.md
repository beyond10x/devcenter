---
format: aep.planning-md/1
id: review-result:recovery-wave-one-refresh-gitlab-authority-pass-1
kind: review-result
status: archived
title: 'Adversarial review: generic connection recovery pass 1'
summary: Two high-severity authority-selection and stuck-session defects require correction.
owner: wave-adversary
relations:
- reviews: story:refresh-gitlab-authority
revision: 2
---
## Verdict

Red. Two high-severity cases introduced by the implementation remain.

## Findings

- `CONN-ADV-001`: when only an app-level callable GitLab grant exists, the curated personal card presents it as the user's callable grant and starts replacement with `gitlab.application` and the deployment grant label. A person-capable provider must select only a matching person profile or `actor: user`; it must never fall back to app authority.
- `CONN-ADV-002`: when connect-session polling fails or exhausts 60 attempts, the cached session remains pending, permanently disabling retry and displaying `Waiting…`. Poll failure and exhaustion must leave an actionable local failed state or clear the pending session.

## Evidence

The focused Connections suite changed from two passing implementation cases to four cases with two failures. The adversary added tests only; production code was untouched.

## Protocol boundary

The released generic `ConnectionRequest` has no revocation operation, and connect-session creation carries no target `connection_ref`; generic revoke and stable-reference replacement remain out of scope until the Connector/ESS contract supplies them.
