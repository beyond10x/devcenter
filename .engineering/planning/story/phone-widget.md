---
format: aep.planning-md/1
id: story:phone-widget
kind: story
status: draft
title: Mount the softphone widget in Devcenter
revision: 1
---
# Mount the softphone widget in Devcenter

## Outcome

An engineer opens `/phone` in Devcenter and calls Asterisk in the dev cluster, through a
`phone-server` running beside them.

## What landed

| file | change |
|---|---|
| `frontend/src/features/phone/PhoneView.vue` | mounts `PhonePanel` from `@b10x/phone-widget` |
| `frontend/src/router/index.ts` | `/phone`, lazily — the widget carries a wasm module and a WebRTC stack |
| `frontend/src/env.d.ts` | `VITE_PHONE_ENDPOINT` typed, so the address is configuration and not an `any` |
| `frontend/package.json` | the widget, as a dependency |

**No deployment fact enters this repository.** The control channel's address defaults to
`ws://127.0.0.1:8780` — where `phone-server` listens out of the box — and is overridden by
environment. No cluster address, namespace or hostname appears in source, which is invariant 6.

Verified 2026-09-05: `pnpm lint`, `pnpm check:generated`, `pnpm typecheck`, `pnpm test` and
`pnpm build` all pass, the production build emits the module as
`dist/assets/softphone-<hash>.wasm` (845.41 kB), and `/phone` in review mode renders the panel with
`control.EndpointConfigured` in its log — the realized wasm system instantiated inside this
application.

## Why this is local-only until the widget is published

The dependency is `file:../../widgets/widgets/phone/widget`. Two consequences, both measured rather
than anticipated:

1. **CI cannot install it.** That path exists on one workstation. `pnpm install --frozen-lockfile`,
   the gate's first line, has nothing to resolve.
2. **`pnpm format:check` fails after any local install.** A `file:` dependency is re-resolved on
   every install, and pnpm writes `pnpm-lock.yaml` in its own single-quoted style while this
   repository's committed lockfile is Prettier-formatted. `prettier --write pnpm-lock.yaml` fixes
   it until the next install.

Neither is a defect in the widget or in this application; both are the `file:` specifier.

## What closes it

`@b10x/phone-widget` pinned by commit, the way `@b10x/agentide-ui` and `@b10x/service-console-vue`
are. Two things have to be settled first, and they belong to the widget's own repository
(`beyond10x/widgets`, `widgets/phone/widget`):

- pnpm's syntax for a package that is not at a repository's root;
- the built `dist/` and the wasm being inside whatever is resolved, because pnpm runs `prepare` on a
  git dependency and this repository's CI has no cargo or `wasm32-unknown-unknown` toolchain to
  build them with.

## What does not work yet, and why it is not this repository's

The phone dials, shows call state and records history. **It carries no audio**: a browser leg that
never completes its bring-up hangs `phone-server` silently, which is
`story:a-browser-leg-that-never-comes-up-hangs-the-phone` in the widgets repository. Nothing in
Devcenter changes when that is fixed.

## Acceptance

`/phone` is reachable in a deployed Devcenter, its dependency is a pinned commit, and
`pnpm --dir frontend install --frozen-lockfile && pnpm --dir frontend check` passes on a machine
that has never seen the widgets repository.
