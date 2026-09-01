---
format: aep.planning-md/1
id: story:canonical-release-names
kind: story
status: implemented
title: Canonicalize release resource names
summary: A release named devcenter must not generate devcenter-devcenter resources.
relations:
- derived_from: epic:shared-secrets-deployment
revision: 5
---
## Acceptance

For release devcenter, every newly rendered workload, Service, ServiceAccount, ConfigMap, ingress, and policy uses one devcenter prefix. Existing persistent claims may be selected explicitly during a source-preserving transition. An explicit Connectors service-account name is reflected in both the pod and its exact Secrets workload grant.
