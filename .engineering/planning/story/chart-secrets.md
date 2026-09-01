---
format: aep.planning-md/1
id: story:chart-secrets
kind: story
status: implemented
title: Add Secrets to the Devcenter chart
summary: Compose custody, auth, and optional development persistence.
relations:
- derived_from: epic:shared-secrets-deployment
- delivers: story:deploy-engineer-journey
revision: 4
---
## Acceptance

The chart renders Secrets with an immutable image, database DSN, read-only keyring, exact workload grants, projected Connectors token, TokenReview RBAC, persistence, and restricted PostgreSQL access.
