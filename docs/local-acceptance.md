# Local deployment acceptance

`devcenterctl local up` builds selected components and runs the actual Helm composition in a dedicated k3d cluster. It then checks normal Identity login, Connector credential custody and authorization, Git materialization, Files navigation, editor layout and edit/save/restore/reload, actual PTY input/output and termination, real Claude replies in main Agents, project chat and coding chat, history pagination beyond the first page, agent/profile editing and removal, and separate durable conversations with real model context isolation. A healthy pod alone does not pass acceptance.

Claude authorization and model requests use the real provider. No synthetic model credential is inserted and the fixture server has no model route. Identity, Connectors, Secrets, Workspace, Substrate, Agent Platform and the BFF run their real service implementations. Upstream OIDC remains a local fixture. Choose `--connector-mode live` to preserve real provider configuration from the private deployment, or `--connector-mode fixture` for the Git forge fixture. The retained selection is recorded separately from the live model mode. A fixture pass does not establish real provider authentication or access; missing authorization and real provider errors leave live acceptance incomplete.

## Resume an existing development session

Read `AGENTS.md` and, when present, the private `.devcenter/local-development.md` handoff before creating resources. Reuse its managed source checkout, local state directory, CLI and Buildx builder. The state directory owns the cluster identity, registry, certificate authority, databases and saved Identity session; changing the directory or recreating the cluster loses that continuity. Do not start a fresh installation merely because a conversation was compacted or closed.

Set these paths from the existing handoff or your own explicit setup. Use an already built CLI whose source matches the local orchestration you intend to test; rebuild it when that source changes.

```bash
"$DEVCENTERCTL" local doctor --state "$LOCAL_ACCEPTANCE_STATE"
"$DEVCENTERCTL" local test \
  --state "$LOCAL_ACCEPTANCE_STATE" --source "$DEVCENTER_CHECKOUT"
```

`local test` checks the running composition and runs the current checkout's browser acceptance suites. It does not build edited application source, reinstall the chart or repeat setup. It reuses `last-project.json` and its private storage-state file only for the recorded node and Connector mode. Keep the referenced storage-state file when cleaning old evidence directories. If the session or node is no longer valid, diagnose that refusal and renew setup through the normal flow.

Choose the next action from what actually changed:

| Change or observation                                 | Next action                                                                                                                                                                                        |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Owner reconnects Claude; runtime is unchanged         | Run `local test` against the retained installation.                                                                                                                                                |
| Acceptance assertions or browser readiness change     | Run the changed acceptance against existing images with `local test`; preserve the real behavior assertions.                                                                                       |
| Frontend or BFF application code changes              | Run focused source checks, then `local up` with `--build server`.                                                                                                                                  |
| Composed Connector code or dependencies change        | Run its locked source checks, then `local up` with `--build connectors`. Select both builds when both are affected.                                                                                |
| Agent Platform implementation changes                 | Run its source checks, then `local up --agent-platform-source "$AGENT_PLATFORM_CHECKOUT"`; the CLI builds and pins that local checkout without requiring a release.                                |
| CLI orchestration, chart or composition values change | Rebuild the CLI if needed, then rerun `local up` with matching baseline inputs and only necessary application builds.                                                                              |
| Immutable released images need verification           | Supply the released image selections in matching private baseline values and lock; omit application build flags to exercise those published images. Retain the documented fixture-CA requirements. |
| Documentation or planning changes only                | Check the text, links, planning store when changed, and required repository gates; retain prior runtime evidence without a new application build or deployment.                                    |

Every `local up` starts from its supplied baseline. Omitting `--build server`, for example, selects the server in that baseline; it does not mean "keep whichever server is currently running." Carry forward the other validated candidate digests in matching private inputs when rebuilding only one component. Keep the same builder and its caches. `--identity-source` requests an Identity build, so omit it on ordinary repeats once the baseline retains the required Identity image.

## Inputs and prerequisites

Use the repository-pinned Node and pnpm versions, Rust, Docker with Buildx, k3d, kubectl, Helm, OpenSSL and Git on an amd64 Linux host with AppArmor and loop-device support. Install frontend dependencies with `pnpm --dir frontend install --frozen-lockfile` and Chromium with `pnpm --dir frontend exec playwright install chromium`.

Supply a complete private baseline values file and matching immutable deployment lock from your deployment repository. Source and chart remain generic. The baseline must enable the hosted workspace, container terminal profile and service composition used by the engineer journey. Application images require access to their registry.

Reserve 12 GiB of memory for the single local node. Keep at least 20 GiB free before builds and monitor concurrent compiler activity; cold dependency builds require additional space. Pause new builds and reclaim reviewed disposable caches when free space approaches 15 GiB. The node evicts workloads below its 10 GiB free-space threshold, which can invalidate pending connection forms; reaching that threshold is a failure, not usable build headroom. After a resource-pressure event, check pod restarts and ownership state as well as current readiness. The node also uses a separate 4 GiB project-quota filesystem. The local node image includes the real AppArmor parser and Substrate's versioned execution-profile prerequisites. Application confinement stays enabled.

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

Administrative credential provisioning uses the existing provider-neutral contract. An external client discovers active integrations and declared requirements with `GET /admin/integrations`, then writes a named credential with `PUT /admin/integrations/{integration_ref}/credentials/{credential}`. The response confirms custody without returning credential bytes. Identity authentication, operator admission, auditing and Connector-owned secret custody remain mandatory. This surface configures only credentials that an activated integration declares as administrative requirements. A catalog provider's endpoint binding alone does not add such a requirement or provision a connection.

The normal `connectors admin integrations` and `connectors admin credentials set` commands expose the same contract and accept a short-lived Identity access token from an owner-only file. Inspect their `--help` for the installed CLI's input flags.

The local fixture writes a provisioning manifest containing integration names, credential names and private value-file paths. The acceptance client discovers requirements, applies each item through the generic endpoint, and checks custody afterward. GitLab is one manifest entry, rather than a special credential route or environment-variable bypass. Provider protocol emulation remains a separate fixture concern. User-bound model credentials and user OAuth connections follow their own normal APIs.

Catalog profiles that declare one secret credential entry use the principal-owned Connect Session flow. A CLI candidate with hosted token setup can submit an existing token from an owner-only file through this same flow. Set `CONNECTORS_CLI` to that candidate executable; the installed release may support only local `setup connect`. Keep the local login's non-secret selection separate from your normal remote CLI selection:

```bash
XDG_STATE_HOME="$PRIVATE_LOCAL_CLI_STATE" SSL_CERT_FILE="$LOCAL_ACCEPTANCE_STATE/ca.crt" \
  "$CONNECTORS_CLI" session login "$LOCAL_CONNECTORS_API_BASE"
XDG_STATE_HOME="$PRIVATE_LOCAL_CLI_STATE" SSL_CERT_FILE="$LOCAL_ACCEPTANCE_STATE/ca.crt" \
  "$CONNECTORS_CLI" setup connect "$PROVIDER" --target hosted \
  --auth-profile "$AUTH_PROFILE" --credential-file "$PRIVATE_PROVIDER_TOKEN_FILE"
```

Use the exact public Connector API base, including the application prefix. The saved Identity login remains in normal OS keyring custody; no browser cookie or upstream credential is copied from another client. The CA file establishes certificate trust without disabling hostname or certificate verification. Token files must be regular, owned by the invoking user, have no group or other access, and contain at most 8192 bytes. Credential bytes must not appear in command arguments or environment variables.

Hosted token setup submits once to the exact Connector-issued completion route and confirms the resulting owner-scoped connection. An unconfirmed submission may already have stored the credential; inspect connections before starting another session. A callable connection still requires a fresh admitted provider read for local acceptance. OAuth consent and native acquisition requiring multiple fields use their declared flows.

For example, `grafana.service_account_token` needs an existing Grafana service account token. A person's SSO password is not that API credential. Token creation and its permissions belong to the deployment's Grafana administration or provisioning; the Connector setup flow does not create the service account. Automation can call the reusable `HostedClient::connect_with_credential_file` with a normally issued short-lived Identity bearer and owner context, without requiring a desktop keyring. Private provisioning references stay in the downstream deployment repository.

### Real integrations from the private deployment

Use a matching private values/lock pair containing actual provider configuration. Values already rewritten for the Git fixture cannot be a live GitLab baseline. Preserve the retained local Identity image and CA when composing candidate images with those private provider declarations.

```bash
"$DEVCENTERCTL" local up \
  --state "$LOCAL_ACCEPTANCE_STATE" --source "$DEVCENTER_CHECKOUT" \
  --baseline-values "$PRIVATE_BASELINE_VALUES" --baseline-lock "$PRIVATE_BASELINE_LOCK" \
  --docker-config "$PRIVATE_DOCKER_CONFIG" --k3s-image "$PINNED_K3S_IMAGE" \
  --connector-mode live --repository-ref "$REAL_REPOSITORY_REF" \
  --build server --github-token-file "$PRIVATE_GITHUB_TOKEN_FILE"
```

Complete GitLab, Slack and other provider acquisition through their normal Connector browser forms. Provider availability and endpoint bindings come from the private deployment; enabling live mode does not manufacture credentials or override grants. Optional `--provisioning-file "$PRIVATE_PROVISIONING_MANIFEST"` supplies the existing generic administrative credential flow for integrations that declare administrative requirements. It never substitutes a model credential. Without this option no administrative credential write is attempted in live mode.

Private provider origins need both Connector destination admission and Kubernetes egress access. The chart's public HTTPS rule excludes private address ranges. Declare the required destination CIDRs and ports in the private baseline's `networkPolicy.extraEgress`; live preparation preserves those rules and adds the local ingress-controller route without duplicating it. If exact destination addresses change, reconcile the private declaration. Verify reachability from a pod governed by the application's policy after policy enforcement has converged; host or node-network success does not prove pod access. A probe started immediately with a new pod can run before the network-policy controller installs its rules.

`integrations.local.json` retains the selected mode, repository reference and optional manifest path. Omitted options reuse them. Explicitly changing modes clears the previous mode's manifest and repository selection. Live mode never imports the fixture provisioning manifest. Preparation, application and test attempts invalidate previous acceptance receipts. Each new receipt records both `provider_mode: live` and `connector_mode: live|fixture`.

The integration suite derives its required providers from the composed private configuration (native GitLab, Slack and Grafana sections and generic catalogue bindings). For each provider it searches admitted operations, obtains a fresh description, and invokes a read-only operation that accepts an empty input object. Missing authorization, a missing suitable read, or an upstream refusal fails that provider check. Evidence retains only statuses and Connector audit references, never provider response bodies. This is additional to repository materialization and model acceptance.

For credential-custody changes, include interleaved acquisitions and interrupted-save recovery against populated state. A fresh empty store cannot reveal retirement or recovery conflicts between providers. A rendered Connect form and an unauthenticated health response do not establish successful acquisition: verify credential persistence and an admitted provider read. If a real provider credential is unavailable, record that gap and keep that provider acceptance incomplete.

Exercise pending status through the BFF before completing authorization, and verify that the UI observes completion and displays the saved connection after reload. A failed status read leaves the outcome unconfirmed; recovery checks the same session without submitting credentials again. Describe and invoke provider operations in separate authenticated requests through the public hosted backend. A test that retains a private internal adapter can hide request-to-request state loss in the actual service.

The lifecycle suite uses the deployed APIs and browser UI, including real Claude turns to prove that a conversation remembers its own previous turn and that a new or cleared conversation does not. Test-created agents and profiles are removed through the normal APIs. Re-run browser acceptance against unchanged candidate images with `local test`; retain the private session file it references.

## Cleanup and iteration

Follow a short feedback loop: reproduce one failing phase, make the bounded correction, run the relevant source checks, rebuild only changed components, and exercise the affected journey locally. Run the complete required source gate and local acceptance before publishing a changed application. After publication, verify the actual immutable images locally, render the matching private deployment inputs, promote through the existing deployment workflow and verify the remote journey. A compilation pass cannot establish service startup, authorization or browser usability.

Report which phase failed and what passed independently. The acceptance runner continues independent model, history and workspace checks after setup, but any failed suite leaves the overall result failed. A missing credential requires the normal Connections authorization flow followed by fresh replies; repeating a build does not repair it. Record build, rollout, workspace-readiness and reply durations separately so a slow phase gets investigated directly.

For workspace automation, wait for an actual repository file such as the README before switching panes. An API Ready result can precede the browser's next poll; the initial explorer also contains a Load workspace placeholder. Hidden progress and network-idle checks alone do not prove the editor has initialized. Preserve the exact save/restore, fresh nonce reply and binary PTY assertions when correcting timing.

Use `devcenterctl local doctor --state "$LOCAL_ACCEPTANCE_STATE"` to inspect node readiness and resource pressure. Use `devcenterctl local down --state "$LOCAL_ACCEPTANCE_STATE"` to remove only recorded local resources. This deletes the local test databases and workspaces. Logs, evidence and the protected node-profile verification files stay in the run directory. Keep the latter while its versioned AppArmor profile remains installed in the shared host kernel. A later `local up` creates a fresh installation from the same explicit inputs.

Use a short, task-owned `TMPDIR` when the shared temporary filesystem is exhausted; Unix socket tests can fail if the directory path is too long. If a shared compiler wrapper still writes into an exhausted temporary directory, disable it for the affected command with `RUSTC_WRAPPER=` and bound compilation with `CARGO_BUILD_JOBS=4`. Keep source-test caches separate from image caches and preserve their provenance.

Cold builds include dependency compilation and image pulls. Subsequent builds reuse caches; select only changed components, then rerun the same real-service journey. Keep failed evidence when diagnosing regressions. Remove ephemeral build credentials after use, and review specific inactive build caches before reclaiming space. Avoid broad volume or system pruning.

Before pausing, update the already ignored `.devcenter/local-development.md` in the checkout the next session will open. Record the retained state path, managed source checkout, CLI executable and provenance, builder, private baseline paths, latest evidence/result, any protected or owned resources, and the exact next command. Store paths and non-secret observations only; never copy cookies, authorization codes, tokens, rendered Secrets or private keys into the note. Keep this machine-specific handoff outside Git and the public documentation allowlist. Version reusable process changes in this guide and link them from `AGENTS.md`.
