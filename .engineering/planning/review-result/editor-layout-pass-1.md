---
format: aep.planning-md/1
id: review-result:editor-layout-pass-1
kind: review-result
status: active
title: Editor layout and production browser acceptance review
relations:
- reviews: story:projects-connection-recovery
revision: 1
---
approve

No remaining actionable findings in the corrected runtime, tests, and release metadata reviewed against `321880bd8a0b8c0f316046f3df4271012fd98fe4`. Recommend accepting this implementation, with the remaining repository gates and deployed acceptance completed separately.

Independent comparison proves the runtime CSP is exactly its predecessor plus `style-src-attr 'unsafe-inline'` (`crates/devcenter-http/src/lib.rs:1009`). This allows style attributes throughout the application document, not just inside Monaco. It preserves the nonce requirement for inline script and style elements, existing same-origin script/style sources, and every network, font, form, framing, object, and base restriction. The distinction between style attributes and style elements follows the [CSP Level 3 directive semantics](https://www.w3.org/TR/CSP3/#directive-style-src-attr). The existing nonce generation and HTML substitution remain unchanged. This intentional CSS-attribute relaxation does not grant inline JavaScript execution or add a network destination; it is not a CSS sanitizer. Existing Markdown rendering still disables raw HTML (`frontend/src/components/RenderedMarkdown.vue:9`).

The Vite change disables ordinary small-asset inlining (`frontend/vite.config.ts:17`), allowing bundled font requests to satisfy the unchanged same-origin font policy. This is consistent with [Vite's asset-inlining option](https://vite.dev/config/build-options.html#build-assetsinlinelimit) and the pinned implementation inspected locally. It affects eligible small assets generally, so it can introduce additional cacheable asset requests beyond fonts. Inspection of the generated CSS found 13 font URLs, all under the existing same-origin asset route, and no font data URLs. Asset serving, dependency versions, and build output directory are unchanged.

The browser fixture now builds and serves production assets, with server reuse disabled (`frontend/playwright.config.ts:20`). The editor test captures policy violations from document initialization, verifies at least three actual line rectangles have the configured height and do not overlap, checks token colors, and inspects nonce-bearing style elements while the editor is mounted (`frontend/e2e/devcenter.spec.ts:1386`). That avoids the previous possibility of checking styles only after leaving the editor. It also exercises edit/save and checks that no policy violations accumulated through navigation and reload. The retained predecessor run fails the new geometry assertion; the corrected production-preview run passes.

The fixture still mocks APIs and injects its own CSP with a fixed nonce. Its script, style, attribute, font, connection, and image directives match the relevant runtime restrictions, but it does not exercise the Rust response or every runtime directive. The Rust response assertion independently checks the actual header's sole unsafe-inline occurrence and nonce substitution (`crates/devcenter-http/src/lib.rs:6178`). The browser geometry regression is desktop Chromium coverage, not evidence across all browsers or production deployment.

The initial forbidden non-null assertion was removed without behavioral change. Production preview also exposed the existing contrast helper's six-digit-only hex parser: minified CSS includes three-digit colors. The final test expands three-digit hex before the same luminance calculation and retains the 4.5 threshold (`frontend/e2e/devcenter.spec.ts:1732`). Both observations are resolved in the reviewed diff. The final frontend gate passes formatting, lint, generated-type checks, type checking, 46 unit tests, and production build. The final browser gate passes 29 cases with 15 intentional skips; focused lint and formatting also pass after the contrast adaptation. The reviewer inspected logs without duplicating builds or tests. Remaining repository gate completion was not established at this review snapshot.

Parsed metadata confirms server candidate 0.8.29 is consistent across root Cargo, frontend, and OpenAPI metadata. Only eight local root package versions change in Cargo.lock. Frontend dependency lock, Connector composition, chart, and CLI runtime source are unchanged. Existing release normalization excludes local version bookkeeping from content impact (`crates/devcenterctl/src/publication.rs:238`), supporting reuse of unchanged release units. No deployment configuration or credential changes are introduced.

The reviewed diff for `CHANGELOG.md`, `Cargo.lock`, `Cargo.toml`, `crates/devcenter-http/src/lib.rs`, `frontend/e2e/devcenter.spec.ts`, `frontend/package.json`, `frontend/playwright.config.ts`, `frontend/vite.config.ts`, and `openapi.json` has SHA256 `65470645fa0bbdc94a088263ca9262e653a705e930107086f9e71cebc30361c3`. Whitespace validation passes. Retained live diagnostics independently show successful save and exact restoration alongside enforced font/style-attribute violations and affected line geometry; they establish the predecessor problem, not deployment of this correction. Immutable publication, deployed policy/geometry verification, and final delivery acceptance remain separate evidence. Planning records and browser credential storage were outside this read-only review.

```findings
[]
```
