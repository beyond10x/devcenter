---
format: aep.planning-md/1
id: review-result:git-http-auth-consumption-pass-1
kind: review-result
status: active
title: Independent Git authentication consumption review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
# Independent Devcenter Git-auth consumer review

## Findings

Parser-compatibility rendering of the same initial judgment: the installed repository-compatible AEP parser subsequently refused `high` and accepts exactly `blocker`, `warning`, or `note`. This new immutable rendering uses `blocker` in the findings JSON while retaining the original high-severity R1 judgment in prose and the unchanged finding message. It does not perform a new review or reconsider the later remedy. The prior field-corrected rendering remains unchanged, SHA256 `5aa35c7b087914fdc5676616361f6fd8a08a7e1413bf642c37366f765a8f1543`; the original report remains unchanged, SHA256 `4beeb09af2cbd201c89011784299500b425010017c75903dfe3d69958d36f5f7`. All other findings fields are identical to the prior rendering.

This complete schema-admissible restatement preserves the original high-severity R1 judgment at commit `a8ddfc48ef2cc1ed968bc555a73fed5ce9c52bd7`; it does not reevaluate or retroactively clear that finding. The original immutable report is `devcenter-git-auth-review.md`, SHA256 `4beeb09af2cbd201c89011784299500b425010017c75903dfe3d69958d36f5f7`. AEP refused its machine block because `id` was unsupported; this new report changes only that block to the admitted field names and adds this provenance note. The original report remains unchanged.

One blocking finding. Severity: high (delivery).

**R1 — The new SSH dependency cannot use the Connectors image builder's token.** `crates/devcenter-connectors/Cargo.toml:22` now selects `ssh://git@github.com/beyond10x/connectors.git`. Host CI supports that transport (`.github/workflows/gate.yml:59`, `.github/workflows/release.yml:95`), but its Git configuration is outside the isolated build container. The actual Bake target selects `Dockerfile.ess`; its Connectors stage at `Dockerfile.ess:45` and authoritative command at `ess/build.yaml:129` install only an HTTPS-prefix rewrite. The maintained `Dockerfile:44` has the same limitation. Neither Connectors build stage has an SSH credential/agent mount or inherited host Git configuration. A clean build therefore attempts SSH without the credential supplied as the HTTPS token. Local metadata resolution does not validate this image boundary.

I reproduced the URL-selection distinction without contacting any remote: with global/system configuration disabled, a synthetic HTTPS-only `insteadOf` rule and `git ls-remote --get-url` leave the SSH source URL unchanged. This is source and URL-resolution evidence; I did not execute or observe a failed image build.

The repair must make the existing token transport cover the selected source inside the Connectors builder, or choose a correctly supported source identity. Update the authoritative ESS recipe and its projections together if changing that recipe; retain correct secret handling and locked builds. The current classifier treats ESS recipe changes as shared and Dockerfile changes as affecting every image unit (`crates/devcenterctl/src/publication.rs:183`), so any broadened repair needs truthful impact selection rather than assuming the present connectors-only result remains valid.

## Scope and dependency evidence

Reviewed commit `a8ddfc48ef2cc1ed968bc555a73fed5ce9c52bd7`, based on `ab201760b3cdcd02abda72881dcada1080e5644a`. The two-file diff for `crates/devcenter-connectors/Cargo.toml` and `Cargo.lock` has SHA256 `622b935b8fc7271cf06e0bfce309b0095d5b1e25a2040d4902b183ad1b01ec6a`. Planning, publication and any subsequent repair are outside this initial review hash.

Independent parsed lock comparison passed: the package set is unchanged. Differences are exactly composition version 0.8.20 to 0.8.26, 32 Connectors package source substitutions from `235558c11f5fc2e4b8f8440474fb975df49d5329` to `fe3541a6d866e84855dfdc19ec4d22a7e779b1e5`, and the previously reviewed direct Base64 0.22.1 dependency edge for integration-gitlab. No unrelated package version, checksum or dependency changes were found.

The coordinator-provided `devcenter-connectors-metadata.json` parses as one exact Connectors Git source with no duplicate Connectors package names. Its resolve graph gives connectors-runtime, service-connectors and service-catalog the same protocol and service package IDs. Metadata SHA256: `34cc0483c43ff19cc150fe986246571cdf18d32fa15e9ffba7f2b24b86dd1edb`. The retained same-URL patch probe reports Cargo's expected refusal; the SSH source identity fixes that resolver issue but creates R1 at the image boundary.

Direct Connectors Git-object comparisons show the shared contracts are unchanged across the old and selected revisions: `crates/protocol` tree `58e5b2d7f8e6a07e103f83d4e15ed0ad5c243011` and `crates/service` tree `a3d3ad6884105b40d12804771b47806395468f4a` match exactly. The runtime source change is the reviewed Git HTTP authentication repair; remaining intervening changes are records/documentation. This supports the narrow compatibility rationale for the root protocol/service patches, without claiming completed consumer type-checking.

Static review of the existing release classifier confirms the current two dependency files select only connectors, while the planning paths are ignored. Other release-unit versions and artifacts are unchanged in this commit. The complete reviewed Devcenter diff passes `git diff --check`.

## Observed verification and limits

This review used local reads, parsed supplied metadata and both lockfiles, compared local Git objects, inspected the existing CI/Bake/ESS publication path, and ran the network-free URL expansion probe. It made no source/planning edits, builds, remote requests or cluster actions. One whitespace inspection initially used a Devcenter base in the Connectors object database and was refused; it was rerun successfully in the correct Devcenter checkout.

The source PR15 run 34002384974 was reported running at dispatch; I did not query its later state. Source CI, consumer CI, cold image construction, immutable release, deployment and authenticated headless acceptance remain pending evidence for this review. No fixture, metadata or source inspection is represented as a deployed result. No other blocking issue was identified within the assigned scope.

```findings
[
  {
    "file": "Dockerfile.ess",
    "line": 45,
    "category": "correctness",
    "severity": "blocker",
    "message": "R1: At reviewed commit a8ddfc48ef2cc1ed968bc555a73fed5ce9c52bd7, the Connectors image recipe rewrites only HTTPS URLs while the newly selected exact dependency uses SSH. The isolated builder does not inherit host Git authentication and has no SSH credential mount, so a clean image build cannot use its supplied HTTPS token for that source. Update the authoritative Connectors build authentication and projections, or choose a correctly supported source identity; validate the cold image boundary before publication."
  }
]
```

Final immutable initial review; do not amend after delivery.
