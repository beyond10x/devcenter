---
format: aep.planning-md/1
id: epic:shared-secrets-deployment
kind: epic
status: implemented
title: Compose shared encrypted custody
summary: Deploy Secrets as an inner Devcenter service.
revision: 4
---
## Outcome

The public chart composes Secrets, development PostgreSQL, key custody, and least-privilege workload authentication without a standalone service chart.

## Acceptance

Chart lint and rendered-manifest assertions cover embedded and external PostgreSQL, pre-created keyring references, projected audiences, RBAC, persistence, and network isolation.
