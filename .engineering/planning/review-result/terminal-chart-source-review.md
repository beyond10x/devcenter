---
format: aep.planning-md/1
id: review-result:terminal-chart-source-review
kind: review-result
status: active
title: Opt-in terminal chart source review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

Independent read-only review of the four chart files over base `5657c42da29a2b8483d1502b875edd76bd0fe4bf`. No actionable findings in the reviewed opt-in composition. This is source approval, not approval of a private deployment or a claim of successful terminal acceptance.

`substrate.execution.enabled` defaults to false. When enabled, `deploy/charts/devcenter/templates/substrate.yaml:6` requires a seccomp profile path and the explicit amd64 node selector. The schema permits only a relative directory prefix followed by the fixed versioned `host-exec-v1-amd64.json` filename; it excludes absolute paths and parent traversal. The AppArmor name is fixed to the name required by the reviewed bootstrap. Both runtime profiles use Kubernetes Localhost selection, so there is no chart fallback to an unconfined application profile.

The opt-in selects only the fixed `substrate-container-exec` entrypoint. It passes the quota selector before the argument delimiter when existing project quotas are enabled, then preserves the normal daemon arguments and quota-ID checks. The application starts with UID/GID 0 and exactly CHOWN, SETGID, SETUID, SETPCAP and SYS_ADMIN after dropping ALL capabilities. It remains on a read-only root filesystem. This matches the bootstrap's exact preflight and permanent drop to the non-root daemon; the quota variant retains only its existing file-capability path after that transition. The chart does not grant MAC_ADMIN, host namespaces, host paths, privileged-container mode or profile installation authority to the application.

The new temporary directory is a memory-backed emptyDir mounted only for the execution opt-in, with a positive Mi/Gi size. Existing state, runtime, TLS, trust-root and optional workspace-PVC mounts are preserved, as are StatefulSet ownership, replica limits and rollout behavior. The existing finite CPU/memory defaults remain in place. Custom values must retain finite enclosing limits; the bootstrap provides the final refusal if the node does not enforce the required ceilings. The README correctly says to account for temporary storage within the container memory ceiling and to provision the node profiles before rollout.

With execution disabled, the existing ordinary and quota daemon command/capability branches remain. The intentional change common to all Substrate modes is `automountServiceAccountToken: false`. It removes an unused Kubernetes bearer from these pods while preserving explicit image-pull and TLS/trust-root references. No runtime service configuration, model credential path, BFF authority or terminal-admission profile was added. Downstream node installation and terminal admission remain separately owned and reviewed.

The diff contains the three tracked chart changes plus the new chart README; no other non-planning path changed. `git diff --check` passed. The coordinator reports successful chart lint and opt-in rendering. This reviewer inspected the exact templates, defaults, schema and README rather than rerunning render/build commands or opening rendered Secrets. Kubernetes scheduling, node-local profile existence, immutable runtime compatibility, actual post-bootstrap identities, workspace preservation, quota behavior, complete chart/repository gates and deployed terminal acceptance remain separate required evidence. Chart release metadata and downstream immutable selection must be finalized through the existing release/deployment process.

Reviewed file SHA-256 identities:

```text
64faa4ba7ef7a84ab3b7d582d6dd9e1c4f2424d523fdcfb47bb589edc7b0420b  deploy/charts/devcenter/README.md
83f89101058a0c6b6b8ef0e03b612200b377e542b4c743536d20d94c3d6d2891  deploy/charts/devcenter/values.yaml
8a75cd739741279018e135eee79e2b945a3b9d71cb6dc71c3dc669d1cf1b8272  deploy/charts/devcenter/values.schema.json
325eced2c63a9b653270d0d31d5c66861dc2df3e81bba608933077c7b59e3766  deploy/charts/devcenter/templates/substrate.yaml
```

The full-index diff against the base over the three tracked changed files has SHA-256 `cc851b98eab94afb944b591fc61d8060e9235c7ade83538e70f94dbdad2b95a0`. The new README is bound separately by its file hash above.

```findings
[]
```
