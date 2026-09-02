---
format: aep.planning-md/1
id: story:connector-capability-ux
kind: story
status: implemented
title: Clarify connectors and capability permissions
summary: Lead with owned connections and make individual and bulk agent permission posture unmistakable.
relations:
- derived_from: epic:authenticated-control-plane
revision: 4
---
## Outcome

People land on their own Connector connections first and can understand or change an agent capability profile without mistaking an unselected capability for an enabled one.

## Acceptance

- The Connectors workspace opens on My connectors, presents that tab before Catalog, and preserves direct provider/catalog navigation.
- Every capability renders one explicit effective posture: Allow, Approval, or Deny; capabilities absent from the stored mapping render as Deny.
- Postures use accessible text, pressed state, iconography, and distinct non-color-only selected treatments.
- Allow all and Deny all update the selected profile in one revisioned request, preserve Connector-enforced approval requirements, disable during mutation, and report success or refusal.
- Desktop, mobile, accessibility, type, lint, formatting, unit, and production-build gates pass.

## Out of Scope

Automatic permission grants when a new connection appears and changes to Connector or Agent Platform authorization semantics.
