# Local deployment acceptance

`devcenterctl local up` builds selected components and runs the actual Helm composition in a dedicated k3d cluster. It then checks normal Identity login, Connector credential custody and authorization, Git materialization, Files navigation, editor layout and edit/save/restore/reload, actual PTY input/output and termination, replies on both agent surfaces, and history pagination beyond the first page. A healthy pod alone does not pass acceptance.

The external OIDC, Git forge and model endpoints are deterministic fixtures. Identity, Connectors, Secrets, Workspace, Substrate, Agent Platform and the BFF run their real service implementations. This proves their composition with the selected artifacts; live provider credentials and upstream provider behavior still need a separate deployment check.

## Inputs and prerequisites

Use the repository-pinned Node and pnpm versions, Rust, Docker with Buildx, k3d, kubectl, Helm, OpenSSL and Git on an amd64 Linux host with AppArmor and loop-device support. Install frontend dependencies with `pnpm --dir frontend install --frozen-lockfile` and Chromium with `pnpm --dir frontend exec playwright install chromium`.

Supply a complete private baseline values file and matching immutable deployment lock from your deployment repository. Source and chart remain generic. The baseline must enable the hosted workspace, container terminal profile and service composition used by the engineer journey. Application images require access to their registry.

Reserve 12 GiB of memory for the single local node. Leave at least 10 GiB free after builds; cold dependency builds require additional space. The node uses an absolute free-space eviction reserve and a separate 4 GiB project-quota filesystem. The local node image includes the real AppArmor parser and Substrate's versioned execution-profile prerequisites. Application confinement stays enabled.

Create a private run directory and private Docker configuration. Supply registry authentication in that configuration and an owner-only ephemeral GitHub token file for private build dependencies. Tokens travel through BuildKit secrets. Keep these inputs outside the checkout.

## Build, compose and test

```console
cargo build --locked -p devcenterctl
./target/debug/devcenterctl local up \
  --state "$LOCAL_ACCEPTANCE_STATE" \
  --source "$DEVCENTER_CHECKOUT" \
  --baseline-values "$PRIVATE_BASELINE_VALUES" \
  --baseline-lock "$PRIVATE_BASELINE_LOCK" \
  --identity-source "$IDENTITY_CHECKOUT" \
  --build server --build connectors \
  --builder "$LOCAL_BUILDX_BUILDER" \
  --k3s-image "$PINNED_K3S_IMAGE" \
  --docker-config "$PRIVATE_DOCKER_CONFIG" \
  --github-token-file "$PRIVATE_BUILD_TOKEN_FILE"
```

Use an immutable `repository@sha256:…` K3s image. `--identity-source` builds Identity through its shipping Dockerfile. Private upstream test-CA trust requires an Identity revision supporting `IDENTITY_UPSTREAM_CA_BUNDLE`. The composed Connector must support the system CA bundle too. Omit a component's build flag only when its pinned baseline already has the needed behavior.

The CLI reuses build caches and resolves built images to registry digests. Unselected components retain their baseline digests. It records the exact node, registry and HTTPS forwarder container IDs and refuses a different installation at those names. It never changes the default Kubernetes context. Repeating the command upgrades the owned installation and runs acceptance again; each attempt keeps separate logs and evidence. A pinned baseline may also reference previously built local images: the CLI republishes their verified cached digests when recreating the registry. Keep those images in the host Docker cache. If an `OnDelete` StatefulSet still runs an old image, composition verification refuses it; deliberately recreate the local installation when changing those dependencies.

The dedicated names are `devcenter-acceptance`, `k3d-devcenter-registry.localhost` and `devcenter-local-https`. Ports 16550, 18080, 18443, 15000 and 443 bind to loopback. Port 443 provides the strict HTTPS origin required by the Git provider. Existing resources require explicit adoption with their exact IDs; they are never silently taken over.

## View and inspect

Open **https://devcenter.localhost:18443** and use the normal sign-in link. The synthetic upstream signs in the local fixture engineer. Browser automation is always headless and trusts only the generated fixture certificate's public-key fingerprint. For manual viewing, trust the run directory's `ca.crt` in your test browser; the CLI does not change host or browser trust stores. Never install the private key.

Setup creates the fixture agent through the normal UI and reuses the existing project on repeated runs. The history check seeds 40 closed coordination records through the admitted BFF service API and follows real cursors until a record absent from the first page is found.

A successful command writes `last-acceptance.json` pointing to the evidence directory. Each journey writes `result.json`, screenshots, individual HTTP statuses, observed startup timing, binary PTY transport evidence and acknowledged cleanup. A failure returns nonzero and keeps its phase logs. Browser session files remain private and must not be published.

To run just the journey with an existing local session and project:

```console
./target/debug/devcenterctl local test \
  --state "$LOCAL_ACCEPTANCE_STATE" --source "$DEVCENTER_CHECKOUT" \
  --origin https://devcenter.localhost:18443 \
  --storage-state "$PRIVATE_BROWSER_STATE" --project "$LOCAL_PROJECT_ID"
```

## Programmatic Connector setup

Credential provisioning uses the existing provider-neutral administrative contract. An external client discovers active integrations and declared requirements with `GET /admin/integrations`, then writes a named credential with `PUT /admin/integrations/{integration_ref}/credentials/{credential}`. The response confirms custody without returning credential bytes. Identity authentication, operator admission, auditing and Connector-owned secret custody remain mandatory. This surface configures credentials for activated integrations; it does not dynamically install arbitrary provider code.

The normal `connectors admin integrations` and `connectors admin credentials set` commands expose the same contract and accept a short-lived Identity access token from an owner-only file. Inspect their `--help` for the installed CLI's input flags.

The local fixture writes a provisioning manifest containing integration names, credential names and private value-file paths. The acceptance client discovers requirements, applies each item through the generic endpoint, and checks custody afterward. GitLab is one manifest entry, rather than a special credential route or environment-variable bypass. Provider protocol emulation remains a separate fixture concern. User-bound model credentials and user OAuth connections follow their own normal APIs.

## Cleanup and iteration

Use `devcenterctl local doctor --state "$LOCAL_ACCEPTANCE_STATE"` to inspect node readiness and resource pressure. Use `devcenterctl local down --state "$LOCAL_ACCEPTANCE_STATE"` to remove only recorded local resources. This deletes the local test databases and workspaces. Logs, evidence and the protected node-profile verification files stay in the run directory. Keep the latter while its versioned AppArmor profile remains installed in the shared host kernel. A later `local up` creates a fresh installation from the same explicit inputs.

Cold builds include dependency compilation and image pulls. Subsequent builds reuse caches; select only changed components, then rerun the same real-service journey. Keep failed evidence when diagnosing regressions. Remove ephemeral build credentials after use, and review specific inactive build caches before reclaiming space. Avoid broad volume or system pruning.
