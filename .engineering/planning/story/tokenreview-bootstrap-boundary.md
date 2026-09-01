---
format: aep.planning-md/1
id: story:tokenreview-bootstrap-boundary
kind: story
status: implemented
title: Keep TokenReview authority outside namespaced deployment
summary: Let cluster bootstrap own one exact binding while the chart remains composable.
relations:
- derived_from: epic:shared-secrets-deployment
revision: 5
---
## Acceptance

The chart can omit its ClusterRoleBinding without omitting the Secrets service account, projected Connectors token, or workload grants. Default rendering still includes TokenReview RBAC; deployment rendering can disable it and use one exact bootstrap-owned binding. Both forms lint and render, and the private namespaced deployer receives no cluster-wide create permission.
