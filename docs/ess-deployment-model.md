---
title: The ESS deployment model
description: How Devcenter turns independently released systems into one exact, reviewable deployment without rebuilding the whole stack.
b10x:
  schema: b10x-doc-page/v1
  audiences: [operator, evaluator]
  experienceIds: [deploy-operate-products]
  support: preview
  access: public
---

# The ESS deployment model

Devcenter separates building an artifact, publishing it, and selecting it for deployment.
The intent, compatibility decisions,
immutable artifacts, environment obligations, and resulting rollout are separate documents with
exact identities. Each stage can therefore be reviewed and refused before anything reaches a
cluster.

## The compilation lifecycle

This first diagram is conceptual: it shows the typed boundaries in the deployment model, not
Devcenter's concrete build nodes.

```mermaid
flowchart LR
  authored["Repository-owned ESS"]
  build["Typed build IR"]
  releases["Immutable releases + evidence"]
  catalog["Offline release catalogue"]
  lock["Exact stack lock"]
  environment["Private environment bindings"]
  deployment["Exact deployment IR"]
  rollout["Independent Helm releases"]

  authored --> build --> releases --> catalog --> lock
  lock --> deployment
  environment --> deployment
  deployment --> rollout
```

The left side is stable, reviewable intent. CI performs the build in the middle and publishes
immutable release evidence. Resolution and environment compilation on the right decide what may be
deployed; only the authorized reconciler performs the final rollout.

## Devcenter's actual build DAG

This is not a hand-maintained interpretation of the build. ESS generates it directly from the same
validated `ess-build/1` IR that produces Devcenter's BuildKit inputs. CI regenerates the Mermaid
source and refuses any difference between the model, the committed projection, and this page.

<!-- ess-build-graph:begin -->
```mermaid
flowchart LR
  subgraph build_graph["devcenter build graph"]
    n0["connectors-base<br/><small>pinned OCI base</small>"]
    n1["debian-base<br/><small>pinned OCI base</small>"]
    n2["ctl-root<br/><small>run</small>"]
    n3["helm-base<br/><small>pinned OCI base</small>"]
    n4["kubectl-base<br/><small>pinned OCI base</small>"]
    n5["node-base<br/><small>pinned OCI base</small>"]
    n6["oras-base<br/><small>pinned OCI base</small>"]
    n7["rust-base<br/><small>pinned OCI base</small>"]
    n8["server-root<br/><small>run</small>"]
    n9["source<br/><small>source</small>"]
    n10["chart-source<br/><small>copy</small>"]
    n11["chart-package<br/><small>run</small>"]
    n12["chart-artifact<br/><small>artifact</small>"]
    n13["node-source<br/><small>copy</small>"]
    n14["frontend<br/><small>run</small>"]
    n15["rust-source<br/><small>copy</small>"]
    n16["connectors-binary<br/><small>run</small>"]
    n17["connectors-installed<br/><small>copy</small>"]
    n18["connectors-image<br/><small>OCI image</small>"]
    n19["ctl-binary<br/><small>run</small>"]
    n20["ctl-with-binary<br/><small>copy</small>"]
    n21["ctl-with-helm<br/><small>copy</small>"]
    n22["ctl-with-oras<br/><small>copy</small>"]
    n23["ctl-installed<br/><small>copy</small>"]
    n24["ctl-image<br/><small>OCI image</small>"]
    n25["rust-with-frontend<br/><small>copy</small>"]
    n26["server-binary<br/><small>run</small>"]
    n27["server-installed<br/><small>copy</small>"]
    n28["server-image<br/><small>OCI image</small>"]
    n1 --> n2
    n1 --> n8
    n3 --> n10
    n9 --> n10
    n10 --> n11
    n11 --> n12
    n5 --> n13
    n9 --> n13
    n13 --> n14
    n7 --> n15
    n9 --> n15
    n15 --> n16
    n0 --> n17
    n16 --> n17
    n17 --> n18
    n15 --> n19
    n19 --> n20
    n2 --> n20
    n20 --> n21
    n3 --> n21
    n21 --> n22
    n6 --> n22
    n22 --> n23
    n4 --> n23
    n23 --> n24
    n14 --> n25
    n15 --> n25
    n25 --> n26
    n26 --> n27
    n8 --> n27
    n27 --> n28
  end
  subgraph release_outputs["Independent release outputs"]
    o0(["chart<br/><small>Helm chart · devcenter-chart</small>"])
    o1(["connectors<br/><small>OCI image · devcenter-connectors</small>"])
    o2(["deployment-cli<br/><small>OCI image · devcenterctl</small>"])
    o3(["server<br/><small>OCI image · devcenter-server</small>"])
  end
  n12 --> o0
  n18 --> o1
  n24 --> o2
  n28 --> o3
```
<!-- ess-build-graph:end -->

## Systems own their truth

Each component repository describes the system it implements: its semantic operations and
outcomes, its build graph, and the runtime obligations its process exposes. A build graph names
source, pinned toolchain images, executable steps, secret mounts, caches, and release outputs. The
same graph can produce several independent outputs without rebuilding their shared prerequisites.

This keeps ownership local. Identity describes Identity, Connectors describes Connectors, and
Devcenter describes its browser application and backend. A product repository does not copy their
Dockerfiles or reinterpret their requirements. It composes released system descriptions through
their declared seams.

## Releases become evidence, not guesses

The build executor turns a declared output into an immutable release manifest. That manifest binds
one source revision to the exact semantic, build, and runtime digests; immutable image or chart
digests; supported platforms; and required provenance, SBOM, signature, and conformance evidence.
ESS validates those bindings, but deliberately does not build, publish, or deploy anything. CI and
the authorized release service remain the effectful executors.

An image and its Helm chart are separate release units. Changing a chart no longer requires
pretending that application bytes changed, and changing one service no longer requires rebuilding
the rest of the product.

## A stack is a constraint; a lock is a decision

Devcenter's generic stack declares compatible version ranges, required semantic surfaces, rollout
dependencies, and typed external systems. An offline resolver selects only releases whose manifests
and evidence satisfy those constraints and emits an exact stack lock. The lock contains no
"latest", mutable tag, or hidden registry lookup. It is a complete, reviewable answer to *which
released things belong together?*

This separation lets a component release independently while a deployed environment remains
unchanged until its lock intentionally advances. Compatibility policy is visible in the stack;
the selected bytes are visible in the lock.

## Publication follows each output

A repository release does not imply a new image for every process or a new chart. Devcenter's
server, deployment CLI, composed Connectors runtime, and chart are separate publication candidates.
An explicit publication can select one candidate; automatic publication evaluates the candidates
against their own last successful publications.

For example, publishing a server change leaves the chart's version and digest intact. A later
Connectors publication still sees Connectors changes that were pending before the server release.
The comparison starts at the source of the last published Connectors artifact, rather than at the
most recent repository tag. Failed or incomplete publications do not advance that baseline.

| Changed input | Publication consequence |
| --- | --- |
| Browser or server implementation | Evaluate the server image |
| Composed Connectors implementation | Evaluate the Connectors image |
| Deployment CLI implementation | Evaluate the CLI image |
| Chart templates or deployment defaults | Evaluate the chart |
| An input shared by several outputs | Evaluate its dependent outputs |
| Planning or public documentation only | No runtime or chart publication |

Every retained artifact keeps its original source commit, version, and immutable digest. A
composition may therefore contain artifacts from several releases without claiming that they were
all rebuilt from the composition's commit. Reusing an artifact is an explicit identity-preserving
operation; a missing or invalid publication record cannot be treated as proof of reuse.

Workspace owns its runtime image publication in the Workspace repository. Its build, image smoke
checks, signature, and durable release metadata do not require a Devcenter server or chart release.
Workflow, AEP Service, and Substrate follow the same repository ownership boundary. Publishing one
of these components makes an artifact available; changing an environment still requires advancing
the downstream selection.

## Environments bind obligations without owning secrets

The generic stack knows that a workload needs an endpoint, configuration coordinate, secret slot,
or authority audience. It does not know deployment hostnames, credential bytes, cluster identities,
or organization policy. A private environment document satisfies those obligations with endpoint
bindings, references to existing secret objects, service accounts, and a target cluster and
namespace.

Compilation lowers the exact stack lock plus those bindings into an exact deployment IR: one Helm
release per independently deployable service and an explicit rollout dependency graph. Missing
bindings, incompatible releases, mutable artifacts, undeclared secrets, and insufficient authority
are compilation refusals rather than late runtime surprises. The authorized reconciler can then
apply that IR through the target cluster's existing Kubernetes and RBAC boundary.

## Why this changes the release loop

The model replaces a full-stack rebuild with a dependency-aware sequence:

1. A component change rebuilds only the affected release outputs and shared build nodes.
2. The executor publishes immutable artifacts and their evidence.
3. The stack resolver proposes a new exact lock only when compatibility constraints admit it.
4. The environment compiler proves that every runtime obligation still has a binding.
5. The reconciler updates independent Helm releases in dependency order and verifies the result.

The result is both faster and stricter: routine component releases become small, while the complete
product remains reproducible from typed source through immutable artifacts to cluster rollout.

Devcenter currently commits its semantic system and canonical multi-output build graph together
with deterministic BuildKit projections. The runtime-manifest, stack-lock, and private-environment
stages are the next adoption boundary; until they are active, the existing deployment path remains
the operational authority.
