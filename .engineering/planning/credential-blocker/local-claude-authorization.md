---
format: aep.planning-md/1
id: credential-blocker:local-claude-authorization
kind: credential-blocker
status: cleared
title: Authorize the local Claude connection before live model acceptance
relations:
- blocks: story:live-model-local-acceptance
withholds: test_result
revision: 3
---
## Missing evidence

The local k3d composition now uses the real provider and performs no synthetic model credential writes. The prior fixture credential was retired through the normal local BFF disconnect operation. Setup, running-image verification and the real model endpoint check succeeded; the live model gate returned AUTHORIZATION_REQUIRED and an incomplete acceptance receipt.

The operator must complete the ordinary Claude authorization flow in the retained local Connections page. Afterward, local test reuses the recorded Identity session and project without building or reinstalling. The blocker clears only after actual live main Agents, project and coding attempts succeed. Credential presence alone cannot clear it, and no credential bytes or authorization codes should be requested through chat.

Evidence: local-evidence:devcenter-claude-20260907/local-live-reusable-setup.log and local-live-retry.log. The retry took approximately 3.5 seconds to verify configuration and report missing authorization. Upstream OIDC and Git forge remain explicit fixtures; this correction removes model emulation.

## Clearing evidence

Two actual local runs now passed with fresh live Claude replies in main Agents, coding chat and project chat. Evidence: local-evidence:devcenter-local-k3d-20260906/acceptance-1788773149796210203 and acceptance-1788773500893271555. This clears the local authorization prerequisite based on completed tasks, not credential presence. It does not establish remote authorization.
