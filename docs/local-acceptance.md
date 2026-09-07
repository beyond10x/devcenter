# Local deployment acceptance

`devcenterctl local up` builds selected components and runs the actual Helm composition in a dedicated k3d cluster. It then checks normal Identity login, Connector credential custody and authorization, Git materialization, Files navigation, editor layout and edit/save/restore/reload, actual PTY input/output and termination, real Claude replies in main Agents, project chat and coding chat, and history pagination beyond the first page. A healthy pod alone does not pass acceptance.

Claude authorization and model requests use the real provider. No synthetic model credential is inserted and the fixture server has no model route. Identity, Connectors, Secrets, Workspace, Substrate, Agent Platform and the BFF run their real service implementations. Only upstream OIDC and the Git forge remain local fixtures; their production integrations still require separate verification. Real provider errors fail local acceptance before promotion.

## Resume an existing development session

Read `AGENTS.md` and, when present, the private `.devcenter/local-development.md` handoff before creating resources. Reuse its managed source checkout, local state directory, CLI and Buildx builder. The state directory owns the cluster identity, registry, certificate authority, databases and saved Identity session; changing the directory or recreating the cluster loses that continuity. Do not start a fresh installation merely because a conversation was compacted or closed.

Set these paths from the existing handoff or your own explicit setup. Use an already built CLI whose source matches the local orchestration you intend to test; rebuild it when that source changes.

```bash
"$DEVCENTERCTL" local doctor --state "$LOCAL_ACCEPTANCE_STATE"
"$DEVCENTERCTL" local test \
  --state "$LOCAL_ACCEPTANCE_STATE" --source "$DEVCENTER_CHECKOUT"
```

`local test` checks the running composition and runs the current checkout's browser acceptance suites. It does not build edited application source, reinstall the chart or repeat setup. It reuses `last-project.json` and its private storage-state file only for the recorded node. Keep the referenced storage-state file when cleaning old evidence directories. If the session or node is no longer valid, diagnose that refusal and renew setup through the normal flow.

Choose the next action from what actually changed:

| Change or observation | Next action |
| --- | --- |
| Owner reconnects Claude; runtime is unchanged | Run `local test` against the retained installation. |
| Acceptance assertions or browser readiness change | Run the changed acceptance against existing images with `local test`; preserve the real behavior assertions. |
| Frontend or BFF application code changes | Run focused source checks, then `local up` with `--build server`. |
| Composed Connector code or dependencies change | Run its locked source checks, then `local up` with `--build connectors`. Select both builds when both are affected. |
| CLI orchestration, chart or composition values change | Rebuild the CLI if needed, then rerun `local up` with matching baseline inputs and only necessary application builds. |
| Immutable released images need verification | Supply the released image selections in matching private baseline values and lock; omit application build flags to exercise those published images. Retain the documented fixture-CA requirements. |
| Documentation or planning changes only | Check the text, links, planning store when changed, and required repository gates; retain prior runtime evidence without a new application build or deployment. |

Every `local up` starts from its supplied baseline. Omitting `--build server`, for example, selects the server in that baseline; it does not mean "keep whichever server is currently running." Carry forward the other validated candidate digests in matching private inputs when rebuilding only one component. Keep the same builder and its caches. `--identity-source` requests an Identity build, so omit it on ordinary repeats once the baseline retains the required Identity image.

## Inputs and prerequisites

Use the repository-pinned Node and pnpm versions, Rust, Docker with Buildx, k3d, kubectl, Helm, OpenSSL and Git on an amd64 Linux host with AppArmor and loop-device support. Install frontend dependencies with `pnpm --dir frontend install --frozen-lockfile` and Chromium with `pnpm --dir frontend exec playwright install chromium`.

Supply a complete private baseline values file and matching immutable deployment lock from your deployment repository. Source and chart remain generic. The baseline must enable the hosted workspace, container terminal profile and service composition used by the engineer journey. Application images require access to their registry.

Reserve 12 GiB of memory for the single local node. Leave at least 10 GiB free after builds; cold dependency builds require additional space. The node uses an absolute free-space eviction reserve and a separate 4 GiB project-quota filesystem. The local node image includes the real AppArmor parser and Substrate's versioned execution-profile prerequisites. Application confinement stays enabled.

Create a private run directory and private Docker configuration. Supply registry authentication in that configuration and an owner-only ephemeral GitHub token file for private build dependencies. Tokens travel through BuildKit secrets. Keep these inputs outside the checkout.

## Build, compose and test

```bash
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

Setup creates the acceptance agent through the normal UI and reuses the existing project on repeated runs. Open **Connections**, select **Connect Claude** (or **Reconnect Claude**), and finish the real provider authorization flow there. Never paste authorization codes into chat or command arguments. Setup preserves an existing connection. Until actual model requests succeed, the command returns nonzero and the local installation stays available for diagnosis. The history check seeds 40 closed coordination records through the admitted BFF service API and follows real cursors until a record absent from the first page is found.

Every attempt replaces `last-acceptance.json` with an incomplete result before running checks; a previous pass cannot survive a new failed attempt. A successful command records `provider_mode: live` and points to the evidence directory. `composition.json` verifies the running Agent Platform uses the real model endpoint, and `model.json` records the actual main Agents task outcome. Each journey writes `result.json`, screenshots, individual HTTP statuses, observed startup timing, binary PTY transport evidence and acknowledged cleanup. A failure returns nonzero and keeps its phase logs. Browser session files remain private and must not be published.

After authorization, rerun acceptance without building or reinstalling. The CLI reuses the private Identity session and project recorded by setup, verifies they belong to this exact node, and checks the actual composition, main Agents, history, and workspace journey:

```bash
./target/debug/devcenterctl local test \
  --state "$LOCAL_ACCEPTANCE_STATE" --source "$DEVCENTER_CHECKOUT"
```

## Programmatic Connector setup

Credential provisioning uses the existing provider-neutral administrative contract. An external client discovers active integrations and declared requirements with `GET /admin/integrations`, then writes a named credential with `PUT /admin/integrations/{integration_ref}/credentials/{credential}`. The response confirms custody without returning credential bytes. Identity authentication, operator admission, auditing and Connector-owned secret custody remain mandatory. This surface configures credentials for activated integrations; it does not dynamically install arbitrary provider code.

The normal `connectors admin integrations` and `connectors admin credentials set` commands expose the same contract and accept a short-lived Identity access token from an owner-only file. Inspect their `--help` for the installed CLI's input flags.

The local fixture writes a provisioning manifest containing integration names, credential names and private value-file paths. The acceptance client discovers requirements, applies each item through the generic endpoint, and checks custody afterward. GitLab is one manifest entry, rather than a special credential route or environment-variable bypass. Provider protocol emulation remains a separate fixture concern. User-bound model credentials and user OAuth connections follow their own normal APIs.

## Cleanup and iteration

Follow a short feedback loop: reproduce one failing phase, make the bounded correction, run the relevant source checks, rebuild only changed components, and exercise the affected journey locally. Run the complete required source gate and local acceptance before publishing a changed application. After publication, verify the actual immutable images locally, render the matching private deployment inputs, promote through the existing deployment workflow and verify the remote journey. A compilation pass cannot establish service startup, authorization or browser usability.

Report which phase failed and what passed independently. The acceptance runner continues independent model, history and workspace checks after setup, but any failed suite leaves the overall result failed. A missing credential requires the normal Connections authorization flow followed by fresh replies; repeating a build does not repair it. Record build, rollout, workspace-readiness and reply durations separately so a slow phase gets investigated directly.

For workspace automation, wait for an actual repository file such as the README before switching panes. An API Ready result can precede the browser's next poll; the initial explorer also contains a Load workspace placeholder. Hidden progress and network-idle checks alone do not prove the editor has initialized. Preserve the exact save/restore, fresh nonce reply and binary PTY assertions when correcting timing.

Use `devcenterctl local doctor --state "$LOCAL_ACCEPTANCE_STATE"` to inspect node readiness and resource pressure. Use `devcenterctl local down --state "$LOCAL_ACCEPTANCE_STATE"` to remove only recorded local resources. This deletes the local test databases and workspaces. Logs, evidence and the protected node-profile verification files stay in the run directory. Keep the latter while its versioned AppArmor profile remains installed in the shared host kernel. A later `local up` creates a fresh installation from the same explicit inputs.

Cold builds include dependency compilation and image pulls. Subsequent builds reuse caches; select only changed components, then rerun the same real-service journey. Keep failed evidence when diagnosing regressions. Remove ephemeral build credentials after use, and review specific inactive build caches before reclaiming space. Avoid broad volume or system pruning.

Before pausing, update the already ignored `.devcenter/local-development.md` in the checkout the next session will open. Record the retained state path, managed source checkout, CLI executable and provenance, builder, private baseline paths, latest evidence/result, any protected or owned resources, and the exact next command. Store paths and non-secret observations only; never copy cookies, authorization codes, tokens, rendered Secrets or private keys into the note. Keep this machine-specific handoff outside Git and the public documentation allowlist. Version reusable process changes in this guide and link them from `AGENTS.md`.
