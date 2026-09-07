---
format: aep.planning-md/1
id: credential-blocker:claude-subscription-redemption
kind: credential-blocker
status: cleared
title: Restore a redeemable Claude subscription credential for Agent acceptance
relations:
- blocks: story:projects-connection-recovery
- blocks: story:refresh-user-bound-model-credential
withholds: test_result
revision: 5
---
## Missing credential evidence

Authenticated project Agent requests are admitted and start, then terminate with model_credential_unavailable. A fresh normal one-use subscription lease was issued successfully, but its immediate redemption returned subscription-oauth-refused, isolating OAuth refresh or refreshed-record validation. A new post-deployment Agent request reproduces model_credential_unavailable. A stored-record Connected projection is not evidence that the credential can fund an attempt.

## Clearing condition

The connection owner completes the normal Claude reconnect flow in Devcenter Connections. A fresh admitted Agent request must then redeem current authority and produce the expected terminal reply; verify both project chat and coding-Agent surfaces before claiming that product acceptance. Reconnection has been requested from the operator and no completion has been reported. Never manufacture credentials, rewrite stored secrets or treat elapsed time as evidence.

## Independent work

Repository discovery, Git checkout, file transport and workspace/coordination cleanup can be repaired and verified independently. This blocker withholds the successful Agent test result; it does not prevent that work or claim the credential-readiness UI story is implemented.

## Latest deployed observation

After publication 0.8.34 and the successful private validate/deploy/verify pipeline, fresh requests through personal Agents, project Agent and coding Agent still fail at the model credential boundary. The coding request returns model_credential_unavailable after about three seconds; the independent main Agents task returns the same code. Projects, materialization, Files/editor, real terminal execution, original-workspace preservation and owned-session cleanup all pass independently. Evidence is retained at local-evidence:devcenter-remote-20260907/deployment-acceptance-st0yMO/result.json and remote-agents.json.

The current Claude UI offers Disconnect and then Connect Claude, followed by provider approval and submission of the one-time code in Devcenter. The previously requested reconnect action refers to this sequence; there is no button literally named Reconnect Claude. The owner has not reported completion. Keep this blocker open until real provider-backed replies pass on all three Agent surfaces.

## Resolution

The owner reported completing the normal reconnect and successfully testing the deployed application on 2026-09-07. Independent fresh headless acceptance then confirmed exact nonce replies in main Agents, coding chat and project chat on publication 0.8.35. Files, real PTY execution and owned workspace cleanup also passed. Evidence: local-evidence:devcenter-claude-20260907/remote-reconnected/remote-agents.json and deployment-acceptance-iCBsXv/result.json. This observed redemption clears the credential blocker. It does not establish the broader readiness-projection and pre-admission validation requirements of story:refresh-user-bound-model-credential, which remains active.
