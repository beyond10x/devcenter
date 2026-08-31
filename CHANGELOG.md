# Changelog

## 0.1.1 - 2026-09-01

- Require namespace creation to be explicitly requested by `devcenterctl apply`, allowing a
  namespace-scoped deployment identity to upgrade an operator-bootstrapped namespace.
- Prevent an in-tree confidential-marker denylist from matching its own contents.
- Verify package visibility through supported GitHub APIs during release.

## 0.1.0 - 2026-09-01

- Add the generic Devcenter HTTP service with embedded docs and OpenAPI.
- Add the Rust deployment and confidential-marker verification CLI.
- Add the public, configuration-neutral Devcenter Helm chart.
