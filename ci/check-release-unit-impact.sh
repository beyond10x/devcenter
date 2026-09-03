#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
classifier="$root/ci/release-unit-impact.sh"

assert_targets() {
  expected=$1
  shift

  actual=$(printf '%s\0' "$@" | "$classifier" | sed -n 's/^oci_targets=//p')
  if [ "$actual" != "$expected" ]; then
    printf 'expected targets %q, got %q for: %s\n' "$expected" "$actual" "$*" >&2
    exit 1
  fi
}

assert_targets 'server' frontend/src/App.vue
assert_targets 'server' crates/devcenter-http/src/lib.rs
assert_targets 'connectors' crates/devcenter-connectors/src/main.rs
assert_targets 'deployment-cli' crates/devcenterctl/src/main.rs
assert_targets 'server deployment-cli' Cargo.lock
assert_targets 'server connectors deployment-cli' ess/build.yaml
assert_targets 'server connectors deployment-cli' an-unclassified-runtime-surface/new.file
assert_targets '' docs/ess-deployment-model.md .engineering/planning/story/example.md README.md
assert_targets 'server connectors deployment-cli' \
  frontend/src/App.vue \
  crates/devcenter-connectors/Cargo.toml \
  crates/devcenterctl/src/main.rs

all_targets=$("$classifier" --all | sed -n 's/^oci_targets=//p')
if [ "$all_targets" != 'server connectors deployment-cli' ]; then
  printf 'expected --all to select every target, got %q\n' "$all_targets" >&2
  exit 1
fi

if "$classifier" --unknown >/dev/null 2>&1; then
  echo 'unsupported classifier arguments must fail' >&2
  exit 1
fi

echo 'release-unit impact checks passed'
