---
format: aep.planning-md/1
id: review-result:git-http-auth-consumption-pass-2
kind: review-result
status: active
title: Independent review of the corrected Git authentication image build
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
# Independent Devcenter Git-auth consumer review — pass 2

## Findings

No blocking findings remain in the scoped remedy. R1 is addressed by the concrete build-authentication change; this does not retroactively change the initial high-severity judgment. The original report remains immutable, SHA256 `4beeb09af2cbd201c89011784299500b425010017c75903dfe3d69958d36f5f7`; its complete schema-admissible restatement remains immutable, SHA256 `5aa35c7b087914fdc5676616361f6fd8a08a7e1413bf642c37366f765a8f1543`.

Reviewed 2026-09-06 at commit `4e73a173f3c3119f60a88944d3ac52dba63ba0b4`, against base `ab201760b3cdcd02abda72881dcada1080e5644a`. The complete six-file product diff SHA256 is `88b2120022f264290d685fb1b7ab376dcb5c740a32ea40e39c14e3bcc720d49f`: composed Cargo manifest/lock, `ess/build.yaml`, `generated/ess/build.json`, `Dockerfile.ess`, and maintained `Dockerfile`. Planning and later publication records are outside this hash.

## R1 remedy and preserved boundaries

`ess/build.yaml:130` adds the SSH prefix to the same authenticated HTTPS Git URL using `--add`, preserving the existing HTTPS rewrite. Both values are removed by the existing `--unset-all` after successful construction. The secret remains a BuildKit-mounted token; the remedy needs no SSH key or agent, does not embed credentials in manifests/locks, preserves `cargo build --locked --release`, and changes no runtime authority. `Dockerfile:45` applies the same correction to the maintained build path.

Independent parsed comparison confirms that only `connectors-binary` changes in the ESS IR, and only its shell argv changes: exactly one SSH-prefix rewrite is added. Its mounts, base, workdir, network policy and output remain unchanged. The authored YAML argv, generated JSON argv and JSON-form command in `Dockerfile.ess:45` are identical. All other IR nodes/outputs remain unchanged; Bake and graph projections remain byte-identical to the base.

A network-free Git URL-expansion check, with global/system configuration disabled and synthetic values supplied only through process-local configuration, now rewrites both the SSH Connectors source and an HTTPS SDK source to the synthetic HTTPS destination. This directly addresses the missing transport mapping identified in pass 1, without contacting a provider or using a credential.

The originally reviewed composed manifest and lock are unchanged by the remedy. The earlier parsed checks therefore still apply: version 0.8.26, one exact Connectors source at `fe3541a6d866e84855dfdc19ec4d22a7e779b1e5`, 32 Connectors packages, one protocol/service identity shared by runtime and SDK adapters, byte-identical shared contracts, and no unrelated dependency drift. Saved metadata SHA256 remains `34cc0483c43ff19cc150fe986246571cdf18d32fa15e9ffba7f2b24b86dd1edb`.

## CI impact and publication selection

These are separate mechanisms. The path-based CI impact classifier conservatively selects all image units for shared build-source changes, with a chart impact flag for the ESS inputs (`crates/devcenterctl/src/publication.rs:183`). That broader CI coverage is expected here.

The publication planner explicitly filters to the requested unit before selection and preserves other units' verified provenance in `reused` (`crates/devcenterctl/src/publication.rs:460`). Thus planned `--unit connectors` publication is supported without publishing the server, deployment CLI or chart. Semantic comparison also confirms the non-Connectors build nodes are unchanged. I did not execute the release plan: its selected/reused output must still be read back against the final committed tree and successful immutable manifest history. This review does not claim an actual connectors-only plan or release has completed.

## Observed verification and limits

I inspected the full consumer/product diff and planner/build code; independently parsed and compared the authored/generated recipe; verified unchanged dependency/Bake/graph bytes; ran the network-free URL expansion; and observed successful `git diff --check`. The first in-memory IR comparison assumed nodes were a list; after adapting it to the IR's keyed map, all listed assertions passed. No product source was changed by that checker.

Coordinator-run ESS 0.9.2 logs before and after the remedy each show valid system input, 29 nodes/four outputs compiled, and three BuildKit projections produced without execution. I inspected `ci/check-ess-model.sh`, whose comparisons bind those projections to the checked-in files. Retained log SHA256 values: before `6bac8d51d1ee61030dea94c2f3af0be3d314597964d40afbcdcbaa727b2980c3`; after `652e8032959c7a059605d1085854408c47567b417c4a96454d25072a5b24c915`. The coordinator reports both checker exits and version consistency successful; I did not rerun compilation or builds.

No implementation/planning mutation, remote request, container build, release or cluster action was performed by this reviewer. Actual cold OCI construction, source/consumer CI completion, the final explicit publication plan, immutable release, deployment, and authenticated headless file/Agent acceptance remain separate pending evidence. The scoped source-review result does not substitute for any of them.

```findings
[]
```

Final immutable second-pass report; do not amend after delivery.
