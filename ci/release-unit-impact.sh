#!/usr/bin/env bash
set -euo pipefail

server=false
connectors=false
deployment_cli=false

select_all() {
  server=true
  connectors=true
  deployment_cli=true
}

if [ "${1:-}" = "--all" ]; then
  select_all
elif [ "$#" -ne 0 ]; then
  echo "usage: $0 [--all]" >&2
  exit 2
else
  while IFS= read -r -d '' path; do
    case "$path" in
      frontend/* | openapi.json)
        server=true
        ;;
      crates/devcenter-connectors/*)
        connectors=true
        ;;
      crates/devcenterctl/*)
        deployment_cli=true
        ;;
      crates/devcenter-app/* | crates/devcenter-auth/* | crates/devcenter-core/* | \
        crates/devcenter-http/* | crates/devcenter-mcp/* | crates/devcenter-store/* | \
        crates/devcenter-web-assets/*)
        server=true
        ;;
      Cargo.toml | Cargo.lock | rust-toolchain.toml | .cargo/*)
        server=true
        deployment_cli=true
        ;;
      Dockerfile | Dockerfile.ess | Dockerfile.ess.dockerignore | docker-bake.hcl | \
        ess/build.yaml | generated/ess/build.json | generated/ess/build.mmd | \
        .github/workflows/gate.yml | .github/workflows/release.yml)
        select_all
        ;;
      .engineering/* | docs/* | deploy/charts/* | changes/* | \
        .github/workflows/b10x-docs-pages.yml | .github/workflows/promote-*.yml | \
        .dockerignore | .gitignore | AGENTS.md | CHANGELOG.md | LICENSE | README.md | \
        b10x.docs.yaml | ci/* | ess/system/*)
        ;;
      *)
        # Unknown repository surfaces are image-affecting until classified otherwise.
        select_all
        ;;
    esac
  done
fi

targets=()
if $server; then
  targets+=(server)
fi
if $connectors; then
  targets+=(connectors)
fi
if $deployment_cli; then
  targets+=(deployment-cli)
fi

printf 'server=%s\n' "$server"
printf 'connectors=%s\n' "$connectors"
printf 'deployment_cli=%s\n' "$deployment_cli"
printf 'oci_targets=%s\n' "${targets[*]}"
