---
format: aep.planning-md/1
id: runbook:recovery-wave-one
kind: runbook
status: active
title: Run recovery wave one
summary: Coordinate Workflow, generic connection recovery, and project-run completion across isolated owning repositories.
relations:
- designs: release-plan:devcenter-0-8-16
- decides: story:restore-workflow-library
- decides: story:refresh-gitlab-authority
- decides: story:complete-project-workflow-runs
revision: 2
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

| Unit | Branch | Base | Managed tree | Build directory | Scratch | Stage |
|---|---|---|---|---|---|---|
| restore-workflow-library | `impl/restore-workflow-library` | pending remote Workflow head | pending | `target/` in tree | assigned under wave scratch | planned |
| refresh-gitlab-authority | `impl/refresh-gitlab-authority` | pending opening Devcenter commit | pending | `target/` in tree | assigned under wave scratch | planned |
| complete-project-workflow-runs | `impl/complete-project-workflow-runs` | pending remote Workspace head | pending | `target/` in tree | assigned under wave scratch | planned |

## Commit authorization

Approval authorizes one implementation commit per unit, the integration merges, one closing planning-store commit, and the merge into the local base branch; it authorizes nothing else.
