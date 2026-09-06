---
format: aep.planning-md/1
id: credential-blocker:claude-subscription-redemption
kind: credential-blocker
status: open
title: Restore a redeemable Claude subscription credential for Agent acceptance
relations:
- blocks: story:projects-connection-recovery
- blocks: story:refresh-user-bound-model-credential
withholds: test_result
revision: 2
---
## Missing credential evidence

Authenticated project Agent requests are admitted and start, then terminate with model_credential_unavailable. A fresh normal one-use subscription lease was issued successfully, but its immediate redemption returned subscription-oauth-refused, isolating OAuth refresh or refreshed-record validation. A new post-deployment Agent request reproduces model_credential_unavailable. A stored-record Connected projection is not evidence that the credential can fund an attempt.

## Clearing condition

The connection owner completes the normal Claude reconnect flow in Devcenter Connections. A fresh admitted Agent request must then redeem current authority and produce the expected terminal reply; verify both project chat and coding-Agent surfaces before claiming that product acceptance. Reconnection has been requested from the operator and no completion has been reported. Never manufacture credentials, rewrite stored secrets or treat elapsed time as evidence.

## Independent work

Repository discovery, Git checkout, file transport and workspace/coordination cleanup can be repaired and verified independently. This blocker withholds the successful Agent test result; it does not prevent that work or claim the credential-readiness UI story is implemented.

## Latest deployed observation

After server 0.8.29 and Workspace 0.2.22 delivery, a fresh coding-Agent browser request is admitted with HTTP202 and fails model_credential_unavailable within about three seconds. The earlier project-Agent request failed at the same credential boundary, and normal one-use subscription redemption previously returned subscription-oauth-refused. File read/edit/save/restoration and both normal workspace close states now pass independently.

The operator has been notified that Files is ready and that Claude must be reconnected through the normal connection flow before Agent acceptance can be completed. No reconnection completion or successful model reply has been observed, so this blocker remains open. Once reconnection is reported, verify both Agent surfaces through fresh normal requests before clearing it.
