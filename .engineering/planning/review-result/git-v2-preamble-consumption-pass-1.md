---
format: aep.planning-md/1
id: review-result:git-v2-preamble-consumption-pass-1
kind: review-result
status: active
title: Independent review of Git v2 framing source consumption
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
# Independent review: composed Connectors 0.8.27

Recommendation: approve the scoped consumer change, subject to successful source and consumer CI and the integrating agent's publication/deployment verification. No actionable correctness, security, dependency-isolation, or release-impact findings were identified.

This non-interactive, read-only review covers the uncommitted product diff in `crates/devcenter-connectors/Cargo.toml` and `crates/devcenter-connectors/Cargo.lock` against Devcenter base commit `760460152f884eb2339fa538c50e84cedcd4c91e`. Planning mutations are outside the reviewed product diff. The review used the AEP planning skill and an existing admitted review example to preserve the repository's empty fenced findings format; no AEP command or planning mutation was performed.

The manifest advances the independently released `devcenter-connectors` artifact from 0.8.26 to 0.8.27 and changes all four Connectors revision declarations together from `fe3541a6d866e84855dfdc19ec4d22a7e779b1e5` to `53ba51fb744e223e220523ff49f313e1d23d8673`. The direct runtime/service dependencies and the protocol/service patches therefore retain one exact source. Independent byte comparisons establish that the current manifest and lock equal their base versions after only this SHA substitution and composed-package version substitution; no unrelated dependency version, checksum, feature, or dependency-list drift is present.

The supplied Cargo metadata independently inspected during review resolves 32 Connectors packages from that one exact source. It contains one protocol package and one service package, both at the repaired revision, and includes the integration-gitlab implementation. The runtime still disables default features, and the resolved runtime features remain empty. The upstream Connectors crates retain version 0.6.5; 0.8.27 is the composed Devcenter artifact's version. Comparing the old and new upstream commits shows that the only changed crate files are the previously reviewed Git v2 parser and HTTP fixture. Shared protocol/service contracts and the runtime manifest remain byte-identical.

Workspace isolation is preserved: `Cargo.toml:13` excludes the composed crate, while `crates/devcenter-connectors/Cargo.toml:1` declares its own workspace and lock. The root manifest and root lock are unchanged. The revision bump therefore does not repoint the BFF's released Connectors client or alter the server/deployment-CLI dependency graph.

Build transport remains consistent with the retained SSH source URLs. The unchanged authored recipe in `ess/build.yaml:122`, its generated command in `Dockerfile.ess:45`, and the maintained build in `Dockerfile:38` retain both HTTPS and SSH-prefix rewrites to the BuildKit-mounted token's authenticated HTTPS transport. They still build with `--locked --release` and remove the rewrite entries after successful construction. The unchanged gate and publication workflows also retain both dependency transports. This patch introduces no credential values, SSH-key requirement, build-recipe change, or runtime-authority change.

The version and release scope are appropriate. `ci/check-version-consistency.sh` explicitly permits independent Connectors artifact versions while retaining shared Rust-toolchain checks. The path classifier in `crates/devcenterctl/src/publication.rs:183` maps both changed product paths only to the Connectors unit and ignores planning paths. The explicit publication-unit filter at `crates/devcenterctl/src/publication.rs:460` supports selecting Connectors while reusing the other verified artifacts. No server, deployment-CLI, frontend, OpenAPI, or chart version change is required by this diff. A concrete final publication plan still needs verification against committed source and successful immutable history.

Observed verification: independent source/diff inspection, saved-metadata inspection, deterministic manifest/lock comparisons, unchanged-build/root-workspace comparisons, upstream shared-contract comparison, and `git diff --check` all supported the conclusions above. The coordinator reports formatting, version consistency, and release-classifier checks passed; this reviewer did not rerun them or rebuild the consumer. Source CI was pending and consumer CI had not started when the review was dispatched, so this report does not claim either succeeded. Cold OCI construction, immutable publication, deployed image verification, and authenticated file operations remain separate acceptance evidence.

The findings fence below matches the existing admitted empty-list example. The inspected repository validation log reports empty fenced reviews as having no findings block while still reporting the store valid; this review does not invent a replacement schema or claim that diagnostic is repaired.

```findings
[]
```
