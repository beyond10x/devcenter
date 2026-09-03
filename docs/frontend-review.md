---
title: Review the Devcenter frontend locally
description: Run every frontend journey against local sample data without service credentials or private artifacts.
b10x:
  schema: b10x-doc-page/v1
  audiences: [evaluator]
  experienceIds: [evaluate-beyond10x-products]
  support: preview
  access: public
---

# Review the Devcenter frontend locally

This is the public evaluation path. It runs the frontend against process-local sample data, is
visibly marked as review mode, and contacts no Identity, Connector, model-provider, or Agent
Platform service. It needs no beyond10x account, cluster, service credential, or private artifact.

The hosted-workbench sample includes the real vendored Ghostty renderer and terminal framing over a
local review WebSocket. Its prompt is a deterministic protocol emulator, clearly labelled in the
terminal; it does not run commands or stand in for the separate Workspace-to-Substrate acceptance
lane. `pwd`, `ls`, `whoami`, and `top` return labelled sample output so keyboard, ANSI, resize,
multiple-tab, reconnect, detach, and Kill interactions can be reviewed without a host-shell escape.

[Run the credential-free review from the Devcenter guide](/docs/devcenter/#run-locally).
