---
format: aep.planning-md/1
id: story:generated-service-console
kind: story
status: implemented
title: Run generated services in Devcenter
summary: Discover and invoke SDK-generated service consoles through authenticated Devcenter bindings.
relations:
- derived_from: epic:authenticated-control-plane
revision: 5
---
# Story: Run generated services in Devcenter

## Outcome

An authenticated person can discover every generated service composed into Devcenter, open its exact SDK-generated Vue console, send intents, and observe read-your-writes state through the live Connector runtime.

## Acceptance

- The Devcenter Connector composition registers Todo's generated domain factory and the SDK-owned external service-catalog factory independently.
- Deployment overlays activate both services and assign exact grants without adding SDK concepts to Connectors.
- The BFF lists service catalogs and invokes only catalog-declared operations while retaining descriptions, connections, access tokens, and approval evidence server-side.
- The browser imports the exact SDK Vue component and supplies only operation reference, realm-free input, and explicit write confirmation.
- Tenant and user come from verified login; Devcenter supplies an absent optional realm and no realm appears in a URL, query, header, request body, or browser client argument.
- Frontend, Rust, generated types, browser, chart, version, leak, docs, and planning gates pass.

## Out of Scope

Service behavior remains generated from Todo\x27s service definition. Deployment-specific identities and policy remain in the private deployment repository. No SDK or generated-service concept becomes part of Connectors; its generic embedding seam may omit unused optional capability graphs and must fail closed when omitted capabilities are configured.
