---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-11
kind: release-plan
status: active
title: Release DevCenter 0.8.11
summary: Publish remaining AgentIDE workbench fixes on the canonical tagged dependency graph.
relations:
- delivers: story:agentide-v2-workbench-release
revision: 3
---
# Release plan: DevCenter 0.8.11

## Outcome

Publish the remaining hosted-workbench presentation and terminal-replay fixes on top of DevCenter
0.8.10's canonical released contract graph.

## Acceptance

DevCenter 0.8.11 is built from one Cargo source identity for every shared contract, passes the full
repository and rendered-browser gates, and is ready for the Atlas-based coordinator to release and
promote without rewriting the existing 0.8.10 tag.

## Contents

- Consume Agent Platform 0.6.7, AgentIDE 0.2.1, Workspace 0.2.12, and the Connectors v0.5.6 client
  surface through fixed immutable release tags.
- Consume the composed AgentIDE and Todo generated services through their fixed release tags.
- Preserve Service SDK as the generated-service factory and Eventlog as its persistence seam. Keep
  their source identities, including the compatible Connectors runtime identity, aligned with the
  released generated manifests until those upstream manifests publish tag-based dependencies.
- Recreate only the browser Ghostty renderer on a DevCenter theme change, reconnect to the same
  Workspace PTY, and replay retained output without granting a host shell.

## Publication handoff

- Align Cargo, frontend, OpenAPI, chart, composed Connectors, lockfiles, and changelog versions at
  0.8.11.
- Re-run frontend, rendered-browser, Rust, composed Connectors, chart, rollout, version, leak, and
  strict planning gates on the rebased graph.
- Publish the tested branch through the organization bot and hand the exact revision and checks to
  the Atlas-based coordinator before any main merge, tag, release, or deployment.
