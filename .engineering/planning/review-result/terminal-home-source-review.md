---
format: aep.planning-md/1
id: review-result:terminal-home-source-review
kind: review-result
status: active
title: Review explicit non-root home for hosted execution
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

Independent read-only review of the chart HOME correction against `fd5d8198a9b85a2d0b49d265d332496a5c42b56f`. No actionable findings. This is an actual rendered environment correction; it does not depend on an undeclared extraEnv values hook.

`deploy/charts/devcenter/templates/substrate.yaml:119` now sets HOME to `/nonexistent` only when the container execution profile is enabled. Kubernetes supplies that explicit environment value to the root setup process, and the reviewed bootstrap preserves it through the non-root Tini/daemon handoff. It prevents libgit2 from attempting to inspect the runtime-injected root home after the UID transition. The fixed absent path requires neither a writable home nor a new account. It adds no mount, capability, network allowance, writable image path or authority. Existing TLS/Identity environment entries, quota arguments, state/PVC mounts and security profiles are unchanged. The ordinary non-bootstrap startup path receives no new HOME override.

The cause is supported by a narrow executable comparison. `git2-init-root-home.log` records libgit2 failing to stat `/root/.gitconfig`; `git2-init-absent-home.log` records initialization passing with HOME set to `/nonexistent`. The coordinator ran those probes as the admitted non-root UID in the published image. The default-image libgit2 probe also passes. The separate full libgit2-plus-gix diagnostic passes, but required read-only host SSL library mounts to satisfy that helper's ABI; it is supporting diagnostic evidence, not an unmodified final-image transport test. The native libgit2 HOME comparison addresses the actual initialization failure without relying on that ABI workaround.

`ci/check-chart-rollouts.sh:144` and `:155` require the fixed HOME entry in actual chart output for ordinary execution and execution with project quotas. Existing bootstrap, capability, profile, negative-admission and volume-permission checks remain in place. The inspected template condition also confines the new entry to the intended opt-in. The retained chart-check log records all eight volume-permission executions passing; the coordinator reports completion of the chart script, whose new successful assertions are silent. This reviewer did not rerun rendering or tests.

Chart metadata advances only 0.8.31 to 0.8.32 while retaining appVersion and all image references. The README updates the chart version and explains why HOME must be supplied before the bootstrap drops privileges, without claiming a writable home or adding deployment-specific coordinates. Source changes are limited to this environment entry, its two assertions and documentation/version metadata. Planning records are outside this source review.

Recommendation: accept this bounded chart repair for the reproduced root-home regression, subject to the normal source/release gates and fresh deployed Git materialization plus terminal acceptance. The existing generic bootstrap still inherits HOME when used outside this chart; normalizing that value at its fixed exec boundary is a reasonable separate follow-up. No passwd/NSS change is supported by the reproduction. HOME alone does not purport to disable every Git configuration source; the existing isolated transport and configuration-authority controls remain responsible for that boundary. The separate credential failure remains unresolved by this change.

Reviewed source SHA-256 identities:

```text
a7b9325565a052a24baf6339751bee509463b8a1b20801212eac355c7229fde0  README.md
a86e154c026f64dffa6d273f113a7c23c07c7a1b507c1e4ac0b6d655d0d1729e  ci/check-chart-rollouts.sh
4b6b240def1580a1d2cd3ce30004b48717cdc8d4590ded9f920d0ec82fc171d9  deploy/charts/devcenter/Chart.yaml
5c9b5c7865a56d16b8d0b3003c0c485e3638ee2b5d028925b75a82bd14562617  deploy/charts/devcenter/README.md
63711f442c10418ede6f82e7881e3b7aa49d481a5c6a744349b1817ec12945f1  deploy/charts/devcenter/templates/substrate.yaml
```

The full-index binary diff of these five paths against the stated base hashes to `5a2182504be0a2b66ec0ff06752f1553d9717e1cc8c95eb81436c097b09c8e3d`.

Retained evidence SHA-256 identities:

```text
c7a21025ad2ea94ec20fec2fb589c8524c132c85a3dbc5024033485a89e27b47  git2-init-absent-home.log
a5445bfc07467c53ec6539fa1774751b235007194a2a1e0aa4648046d7029fe3  git2-init-root-home.log
c7a21025ad2ea94ec20fec2fb589c8524c132c85a3dbc5024033485a89e27b47  git2-init-published-image.log
35d132c595e3d2d6343bd75021679d8cb828a1a14237e587970b7c0e27865a8d  git-init-published-with-host-ssl.log
1ccbda5f355d9552d2bd93592f511fab110f027ed82e140c1bee63d1714d60cf  chart-home-rollout-check.log
```

```findings
[]
```
