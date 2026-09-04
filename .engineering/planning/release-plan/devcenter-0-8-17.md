---
format: aep.planning-md/1
id: release-plan:devcenter-0-8-17
kind: release-plan
status: active
title: Release Devcenter 0.8.17
summary: Publish and deploy the real AgentIDE Git workspace composition.
relations:
- delivers: story:real-agentide-workspace
revision: 2
---
## Outcome\n\nPublish Devcenter 0.8.17 with the released AgentIDE workbench, durable actor-private surface state, one exact Git-backed Workspace session, and the internal TLS Git byte plane.\n\n## Qualification\n\nThe complete repository gate passes, the bot-authored default-branch commit is tagged 0.8.17, its private images and chart are published, deployment values consume immutable digests, and the dev-cluster canary proves project open, workspace preparation, file/edit/diff/terminal/chat reload, and cross-user refusal.\n