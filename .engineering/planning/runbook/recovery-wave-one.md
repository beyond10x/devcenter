---
format: aep.planning-md/1
id: runbook:recovery-wave-one
kind: runbook
status: implemented
title: Run recovery wave one
summary: Coordinate Workflow, generic connection recovery, and project-run completion across isolated owning repositories.
relations:
- designs: release-plan:devcenter-0-8-16
- decides: story:restore-workflow-library
- decides: story:refresh-gitlab-authority
- decides: story:complete-project-workflow-runs
revision: 14
---
## Authority

**ADP wave skill version 0.7.0.** The session catalog advertised 0.6.2, whose path was absent; the installed `aep-drive` 0.7.0 authority was loaded instead.

The operator explicitly pre-approved selecting and running the recovery waves, including the unit commits, integration merges, closing store commit, and merge to the local base branch. This does not authorize a push, tag, version bump, release, deployment, or unrelated change.

The collaboration harness does not expose plugin-prefixed `subagent_type` dispatch. Units therefore use a general implementation agent with the 0.7.0 unit brief, followed by a fresh general adversary agent carrying the 0.7.0 adversary contract; this is a recorded deviation rather than an implied plugin-agent dispatch.

## Selected recovery units

- `story:restore-workflow-library` — serves the authenticated engineer journey by restoring the service-owned reusable library.
- `story:refresh-gitlab-authority` — serves governed reach by making connection state and repair actionable; implementation must be provider-generic where the current contract permits it.
- `story:complete-project-workflow-runs` — serves the engineer journey by turning accepted project workflows into observable terminal execution.

The Workflow and project-run units are assigned to their owning upstream repositories; the connection-maintenance unit is assigned to Devcenter. Slack activation and model credential redemption remain outside this wave because Slack is access-blocked and the in-flight Connectors ESS migration owns the richer lifecycle state contract.

## Computed selection

Command: `aep artifact waves --kind story --status active --format json`

```json
{
  "waves": [
    {
      "wave": 1,
      "artifacts": [
        {
          "id": "story:agent-management-controls",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http/src/lib.rs"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e/devcenter.spec.ts"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/api/client.ts"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/agents"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/stores/workspace.ts"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/styles/main.css"
            },
            {
              "confidence": "cited",
              "path": "frontend/tests/workspace.test.ts"
            },
            {
              "confidence": "cited",
              "path": "openapi.json"
            }
          ]
        },
        {
          "id": "story:deploy-engineer-journey",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http"
            },
            {
              "confidence": "cited",
              "path": "deploy/charts/devcenter"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/connections"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/projects"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/workflows"
            }
          ]
        },
        {
          "id": "story:impact-aware-oci-gate",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/gate.yml"
            },
            {
              "confidence": "cited",
              "path": "Dockerfile.ess"
            },
            {
              "confidence": "cited",
              "path": "ci/check-release-unit-impact.sh"
            },
            {
              "confidence": "cited",
              "path": "ci/release-unit-impact.sh"
            },
            {
              "confidence": "cited",
              "path": "docs/ess-deployment-model.md"
            },
            {
              "confidence": "cited",
              "path": "ess/build.yaml"
            },
            {
              "confidence": "cited",
              "path": "generated/ess/build.json"
            },
            {
              "confidence": "cited",
              "path": "generated/ess/build.mmd"
            }
          ]
        },
        {
          "id": "story:promote-workflow-runtime",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/promote-workflow.yml"
            }
          ]
        }
      ]
    },
    {
      "wave": 2,
      "artifacts": [
        {
          "id": "story:complete-project-workflow-runs",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http"
            },
            {
              "confidence": "cited",
              "path": "deploy/charts/devcenter"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/projects"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/stores/workspace.ts"
            }
          ]
        }
      ]
    },
    {
      "wave": 3,
      "artifacts": [
        {
          "id": "story:ess-release-model",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/release.yml"
            },
            {
              "confidence": "cited",
              "path": "CHANGELOG.md"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "Dockerfile.ess"
            },
            {
              "confidence": "cited",
              "path": "ci/check-ess-model.sh"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors/Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "deploy/charts/devcenter/Chart.yaml"
            },
            {
              "confidence": "cited",
              "path": "docker-bake.hcl"
            },
            {
              "confidence": "cited",
              "path": "docs/ess-deployment-model.md"
            },
            {
              "confidence": "cited",
              "path": "ess/build.yaml"
            },
            {
              "confidence": "cited",
              "path": "ess/system"
            },
            {
              "confidence": "cited",
              "path": "frontend/package.json"
            },
            {
              "confidence": "cited",
              "path": "generated/ess"
            },
            {
              "confidence": "cited",
              "path": "openapi.json"
            }
          ]
        },
        {
          "id": "story:refresh-user-bound-model-credential",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/agents"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/connections"
            }
          ]
        }
      ]
    },
    {
      "wave": 4,
      "artifacts": [
        {
          "id": "story:refresh-gitlab-authority",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/promote-connectors.yml"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors/Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors/Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/connections"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/projects"
            }
          ]
        }
      ]
    },
    {
      "wave": 5,
      "artifacts": [
        {
          "id": "story:restore-hosted-slack-connection",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": "crates/devcenter-connectors"
            },
            {
              "confidence": "cited",
              "path": "deploy/charts/devcenter"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/connections"
            }
          ]
        }
      ]
    },
    {
      "wave": 6,
      "artifacts": [
        {
          "id": "story:restore-workflow-library",
          "inferred": false,
          "scope": [
            {
              "confidence": "cited",
              "path": ".github/workflows/promote-workflow.yml"
            },
            {
              "confidence": "cited",
              "path": "Cargo.lock"
            },
            {
              "confidence": "cited",
              "path": "Cargo.toml"
            },
            {
              "confidence": "cited",
              "path": "crates/devcenter-http"
            },
            {
              "confidence": "cited",
              "path": "deploy/charts/devcenter"
            },
            {
              "confidence": "cited",
              "path": "frontend/e2e"
            },
            {
              "confidence": "cited",
              "path": "frontend/src/features/workflows"
            }
          ]
        }
      ]
    }
  ],
  "collisions": [
    {
      "a": "story:agent-management-controls",
      "b": "story:complete-project-workflow-runs",
      "path": "Cargo.lock",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:complete-project-workflow-runs",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:complete-project-workflow-runs",
      "path": "frontend/src/stores/workspace.ts",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:ess-release-model",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:ess-release-model",
      "path": "openapi.json",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/src/features/agents",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:restore-workflow-library",
      "path": "Cargo.lock",
      "confidence": "cited"
    },
    {
      "a": "story:agent-management-controls",
      "b": "story:restore-workflow-library",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:deploy-engineer-journey",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:deploy-engineer-journey",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:deploy-engineer-journey",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:deploy-engineer-journey",
      "path": "frontend/src/features/projects",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:ess-release-model",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:refresh-gitlab-authority",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:refresh-gitlab-authority",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:refresh-gitlab-authority",
      "path": "frontend/src/features/projects",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:refresh-user-bound-model-credential",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-hosted-slack-connection",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-workflow-library",
      "path": "Cargo.lock",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-workflow-library",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-workflow-library",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-workflow-library",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    },
    {
      "a": "story:complete-project-workflow-runs",
      "b": "story:restore-workflow-library",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-gitlab-authority",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-gitlab-authority",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-gitlab-authority",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-gitlab-authority",
      "path": "frontend/src/features/projects",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-user-bound-model-credential",
      "path": "crates/devcenter-connectors",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-user-bound-model-credential",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-hosted-slack-connection",
      "path": "crates/devcenter-connectors",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-hosted-slack-connection",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-hosted-slack-connection",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-workflow-library",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-workflow-library",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-workflow-library",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:deploy-engineer-journey",
      "b": "story:restore-workflow-library",
      "path": "frontend/src/features/workflows",
      "confidence": "cited"
    },
    {
      "a": "story:ess-release-model",
      "b": "story:impact-aware-oci-gate",
      "path": "Dockerfile.ess",
      "confidence": "cited"
    },
    {
      "a": "story:ess-release-model",
      "b": "story:impact-aware-oci-gate",
      "path": "docs/ess-deployment-model.md",
      "confidence": "cited"
    },
    {
      "a": "story:ess-release-model",
      "b": "story:impact-aware-oci-gate",
      "path": "ess/build.yaml",
      "confidence": "cited"
    },
    {
      "a": "story:ess-release-model",
      "b": "story:refresh-gitlab-authority",
      "path": "crates/devcenter-connectors/Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:ess-release-model",
      "b": "story:restore-workflow-library",
      "path": "Cargo.toml",
      "confidence": "cited"
    },
    {
      "a": "story:promote-workflow-runtime",
      "b": "story:restore-workflow-library",
      "path": ".github/workflows/promote-workflow.yml",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:refresh-user-bound-model-credential",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:refresh-user-bound-model-credential",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:restore-hosted-slack-connection",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:restore-workflow-library",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-gitlab-authority",
      "b": "story:restore-workflow-library",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-user-bound-model-credential",
      "b": "story:restore-hosted-slack-connection",
      "path": "crates/devcenter-connectors",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-user-bound-model-credential",
      "b": "story:restore-hosted-slack-connection",
      "path": "frontend/src/features/connections",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-user-bound-model-credential",
      "b": "story:restore-workflow-library",
      "path": "crates/devcenter-http",
      "confidence": "cited"
    },
    {
      "a": "story:refresh-user-bound-model-credential",
      "b": "story:restore-workflow-library",
      "path": "frontend/e2e",
      "confidence": "cited"
    },
    {
      "a": "story:restore-hosted-slack-connection",
      "b": "story:restore-workflow-library",
      "path": "deploy/charts/devcenter",
      "confidence": "cited"
    }
  ],
  "unassessed": [],
  "cycles": []
}
```

## Selection path

The store computed every selected Devcenter story into a different wave because their declared Devcenter integration scopes collide. The source units run concurrently only in separate owning repositories; dependency/client integration into Devcenter remains serial on the coordinator branch. No unassessed active story was selected.

## Pre-flight

- Base: exact current remote Devcenter default branch `d43e81d2`.
- Existing linked trees: several managed non-wave trees exist, but no checked-out `wave/*` or `impl/*` branch collides with this wave.
- Disk: 85 GiB free before the measured build.
- Measured Rust build: `cargo test -p devcenter-http --locked --no-run` completed from a cold target in 56.969 seconds and produced a 1.7 GiB target.
- Compiler cache: `/usr/bin/sccache` 0.16.0 is available; every Rust unit receives `RUSTC_WRAPPER=/usr/bin/sccache`. Current cache is 10 GiB with a 42.37% aggregate hit rate.
- Model budget: the operator provided no numeric limit, so the skill default is 4; repository slots bound concurrent implementors to 3.

## Unit records

| Unit | Branch | Base | Managed tree | Build directory | Scratch | Final wave state |
|---|---|---|---|---|---|---|
| starter Workflow resource bundle | `impl/restore-workflow-library` | `f2bc043b` | `workflow-restore-library` | `target/` in tree | XDG cache `b10x-waves/devcenter-0.8.16/wave-one/restore-workflow-library/scratch` | blocked without product-source change: Service SDK has no immutable resource/pre-readiness reconciliation contract; planning record awaits its bot commit |
| populated Workflow library refusal | `impl/workflow-refusal-diagnosis` | `a2a408c9` | `devcenter-workflow-refusal` | `target/` in tree | XDG cache `b10x-waves/devcenter-0.8.16/wave-one/workflow-refusal-diagnosis/scratch` | exact defect proven; speculative Devcenter pagination change reverted; no source commit |
| Service SDK optional projection fields | `impl/optional-projection-fields` | `7c0655f1` | `service-sdk-optional-projection` | `target/` in tree | XDG cache `b10x-waves/devcenter-0.8.16/wave-one/optional-projection-fields/scratch` | green, bot commit `2cb6b103507d843b6aecb0eb1e936f8a4587f866` |
| generic connection authority recovery | `impl/refresh-gitlab-authority` | `a2a408c9` | `devcenter-recovery-connections` | `target/` in tree | XDG cache `b10x-waves/devcenter-0.8.16/wave-one/refresh-gitlab-authority/scratch` | green, bot commit `e5220bdba45c6285a3cdd9429c78e87a55848f8b`, integration merge `a72924613d9b9401c09bdf2106601d2561286985`; generic revoke remains contract-blocked |
| durable project Workflow runs | `impl/complete-project-workflow-runs` | `d8bf3931` | `workspace-complete-workflow-runs` | `target/` in tree | XDG cache `b10x-waves/devcenter-0.8.16/wave-one/complete-project-workflow-runs/scratch` | green, bot commit `04d34872dad5ecaf02ce152789a78ef09e9e630b`; downstream Devcenter pin awaits the Workspace release |

## Commit authorization

Approval authorizes one implementation commit per unit, the integration merges, one closing planning-store commit, and the merge into the local base branch; it authorizes nothing else.

## Coordinator findings

- The reported Workflow library refusal is not caused by Devcenter's requested page size. Adversarial verification established that released Service SDK 0.5.7 admits `1..=1000`, generated Workflow OpenAPI declares maximum `1000`, and invalid pagination would return HTTP 400 `invalid_page`. The speculative `1000` to `100` correction and literal test were removed before commit; the recorded review outcome is fixed.
- Exact real-image reproduction found the defect: Workflow 0.3.5 returns 200 against an empty store, then returns HTTP 500 `service_contract` after one successful `create_workflow`, including across 0.3.4-to-0.3.5 store reuse. `active_revision_id` is optional and absent; Service SDK materialization omits it while validation requires every view field, producing `InvalidProjection`. `dependency-blocker:service-sdk-optional-projection-fields` records the downstream release chain.
- The Service SDK repair was added to this wave because it is the smallest owner change that satisfies the operator-approved Workflow recovery item; no Devcenter mask, direct DB mutation, or deployment skew workaround was added.
- Automatic starter-library reconciliation is independently blocked: Workflow's focused manifest probe proved that released Service SDK 0.5.7 rejects a `resources` field and offers neither immutable resource IR nor a pre-readiness reconciler. The probe was reverted; `dependency-blocker:service-sdk-resource-reconciliation` records the required upstream contract.
- Workflow diagnosis temporarily wrote `/tmp/devcenter-workflow-*` container state outside the assigned scratch directory and removed it before return. This is recorded as an agent-boundary deviation; no repository or persistent external state was changed.
- The first Connections adversary brief omitted the required fenced findings instruction. Its immutable prose-only result was archived and a normalized replacement was recorded; the store retains a validation warning for the archived record rather than rewriting history.

## Gate results

The complete Devcenter gate was run once on integration commit `a72924613d9b9401c09bdf2106601d2561286985`, one command at a time with its own exit status and output inspected:

- `pnpm --dir frontend install --frozen-lockfile`: exit 0.
- `pnpm --dir frontend check`: exit 0; 9 test files and 31 tests passed, then the production build passed.
- `pnpm --dir frontend exec playwright install chromium`: exit 0; Chromium installed, with host-package fallback warnings.
- `pnpm --dir frontend test:e2e`: exit 0; 17 passed and 13 intentional environment-dependent skips.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: exit 0.
- `cargo test --workspace --locked`: exit 0; all executed workspace suites passed.
- nested `crates/devcenter-connectors` fmt: exit 0.
- nested `crates/devcenter-connectors` clippy: exit 0.
- nested `crates/devcenter-connectors` tests: exit 0; 3 passed.
- version consistency: exit 0.
- Helm lint: exit 0; informational icon recommendation only.
- chart rollout check: exit 0.
- leak check: exit 0; no denied material found.

The Workspace unit's full gate passed 33 tests plus formatting, clippy, and diff checks. The Service SDK unit's full `task check` passed, including 86 tests plus documentation, web, AEP, release, and diff checks. Workflow's unchanged source passed formatting, clippy, tests, and deterministic generated-output checks.

## Review value and execution cost

`aep artifact review-value --since 2026-09-04 --format json` reports four Devcenter review records by `wave-adversary`, seven findings, two no-op outcomes, three fixed outcomes, zero escalations, and unknown recorded cost. The archived malformed first Connections review accounts for the discrepancy between review records and enumerable findings.

The collaboration harness retained no token, tool-call, or wall-duration counters for the completed implementor and adversary runs, so each is recorded as unknown rather than zero. Executed focused cases: Connections 9/9 green after two adversarial passes; Workspace 9/9 focused and 33/33 full tests green after two passes; Service SDK 22/22 relevant engine/builder cases green after two passes and the full gate.

## Release boundary

The wave is implementation and local integration, not release. No branch, tag, image, chart, or deployment was pushed or published.

The exact downstream order is:

1. publish and release Service SDK commit `2cb6b103507d843b6aecb0eb1e936f8a4587f866`;
2. update Workflow to that SDK release, regenerate realization format `service-realization-plan/3`, prove the populated library reads successfully, then publish and release Workflow;
3. publish and release Workspace commit `04d34872dad5ecaf02ce152789a78ef09e9e630b`;
4. update Devcenter's immutable Workflow and Workspace references, include the merged Connections change, run the affected qualification, then cut and deploy the Devcenter release.

Automatic starter-library materialization remains blocked on a separate Service SDK resource-reconciliation contract. Connector revoke/stable-reference replacement remains blocked on the in-flight Connector ESS lifecycle contract.
