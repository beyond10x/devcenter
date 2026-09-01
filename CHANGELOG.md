# Changelog

## 0.2.4 - 2026-09-01

- Fix the upstream login callback by exchanging OAuth authorization codes as standard
  `application/x-www-form-urlencoded` data through Identity 0.3.2.

## 0.2.0 - 2026-09-01

- Add generic Identity-backed authorization-code login with S256 PKCE, one-use state, and an opaque
  secure browser session.
- Add an explicit BFF allowlist for session facts, Claude Code connector custody, Agent Platform
  agent management, Task submission and Task event streaming.
- Add the first authenticated engineer dashboard and expanded embedded OpenAPI/documentation.
- Extend the public Helm chart to compose digest-pinned Identity, Connectors and Agent Platform
  services behind internal ingress with deployment-owned configuration and persistent state.
- Keep application source and images private while retaining the configuration-neutral chart as
  the only public release artifact.

## 0.1.1 - 2026-09-01

- Require namespace creation to be explicitly requested by `devcenterctl apply`, allowing a
  namespace-scoped deployment identity to upgrade an operator-bootstrapped namespace.
- Prevent an in-tree confidential-marker denylist from matching its own contents.
- Verify package visibility through supported GitHub APIs during release.

## 0.1.0 - 2026-09-01

- Add the generic Devcenter HTTP service with embedded docs and OpenAPI.
- Add the Rust deployment and confidential-marker verification CLI.
- Add the public, configuration-neutral Devcenter Helm chart.
