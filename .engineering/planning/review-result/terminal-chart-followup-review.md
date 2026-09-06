---
format: aep.planning-md/1
id: review-result:terminal-chart-followup-review
kind: review-result
status: active
title: Terminal chart regression and release preparation review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

Independent read-only review of the chart test, documentation and version follow-up against `5657c42da29a2b8483d1502b875edd76bd0fe4bf`. No actionable findings. The previously reviewed template, values, schema and chart README retain their approved file hashes exactly; the earlier chart review remains immutable.

`ci/check-chart-rollouts.sh:133` adds positive render checks for the explicit bootstrap entrypoint, fixed AppArmor label, selected Localhost seccomp path, exact five bootstrap capabilities, absent service-account token, root setup and bounded memory-backed temporary directory. The negative check rejects privileged mode, host paths, host PID/network, SYS_RESOURCE and Unconfined policy in the selected application manifest. The combined quota/execution case checks both the bootstrap's quota selector and the existing daemon quota arguments. Missing profile, wrong architecture and a traversing profile path must fail rendering under the existing pipefail behavior. These checks exercise the actual chart output and admission failures; they are structural checks, not proof of kernel confinement, private namespaces or successful execution.

The README addition explains the chart 0.8.26/Substrate 0.7.6 prerequisite relationship without adding deployment-specific coordinates. It keeps node profile installation and Workspace terminal admission with the downstream operator. `deploy/charts/devcenter/Chart.yaml:5` changes only the chart version from 0.8.25 to 0.8.26; the existing appVersion remains unchanged, appropriate to this chart-only release. No service image, application package or dependency version is changed by this follow-up.

The coordinator reports the full chart script passed. The retained `chart-rollout-gate.log` records all eight volume-permission executions passing, covering both shared/separate workspace mounts, root/non-root initial ownership and repeated execution. The new shell assertions are silent on success; the retained log alone does not include a separately serialized exit status for each assertion. No chart render, build or executable test was rerun by this reviewer. Actual node profile installation and deployed terminal acceptance remain separate requirements.

Reviewed file SHA-256 identities:

```text
d904b400ed41a2f4ffcba433b9f4037a949a4e5355f0edfe02e26abbc5c50bb5  ci/check-chart-rollouts.sh
d16a05fdbb693d9e5e5c538dc7c318f26414ff485194a5722e549a952d400996  README.md
e7b45a7658595be221844b971850b297f6d66d9725b37db54999b7b728ddfe7b  deploy/charts/devcenter/Chart.yaml
64faa4ba7ef7a84ab3b7d582d6dd9e1c4f2424d523fdcfb47bb589edc7b0420b  deploy/charts/devcenter/README.md
325eced2c63a9b653270d0d31d5c66861dc2df3e81bba608933077c7b59e3766  deploy/charts/devcenter/templates/substrate.yaml
8a75cd739741279018e135eee79e2b945a3b9d71cb6dc71c3dc669d1cf1b8272  deploy/charts/devcenter/values.schema.json
83f89101058a0c6b6b8ef0e03b612200b377e542b4c743536d20d94c3d6d2891  deploy/charts/devcenter/values.yaml
```

The full-index binary diff against the stated base for the listed paths hashes to `268b8dd8f90eef32828614d2625c03ac1eb922f213b3aa2ae630dc6a4dc0b3d4`. The new untracked chart README is not part of that Git diff and is bound by its separate file hash above. Planning files are excluded. The retained gate log SHA-256 is `bcf4c9de09f927e34dd49c2b3644cb98e385ff7a9cc83fccaef91037737c7e1f`.

```findings
[]
```
