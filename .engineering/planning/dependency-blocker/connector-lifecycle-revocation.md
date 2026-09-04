---
format: aep.planning-md/1
id: dependency-blocker:connector-lifecycle-revocation
kind: dependency-blocker
status: open
title: Connector contract lacks generic revocation
summary: Provider-neutral revoke and stable-reference replacement wait for the Connector ESS lifecycle contract.
owner: connectors
relations:
- blocks: story:refresh-gitlab-authority
withholds: test_result
revision: 1
---
## Missing contract

The released generic Connector request surface can create a connect session but exposes no revoke or delete operation, and connect-session creation carries no target `connection_ref`. Devcenter can now distinguish callable, needs-attention, not-connected, and administrator-setup-required states and can start user reauthorization, but it cannot truthfully offer provider-neutral revocation or promise stable-reference replacement.

## Exit condition

The in-flight Connector ESS migration publishes a generic owner-authorized lifecycle operation with explicit connection identity and stable outcomes for revoke/replace. The generated client exposes it; Devcenter allowlists the route and adds provider-neutral tests without handling credential bytes.
