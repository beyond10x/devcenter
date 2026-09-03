#!/usr/bin/env bash
set -euo pipefail

ess_bin=${ESS_BIN:-ess}
expected_version=0.9.2
actual_version=$($ess_bin --version | awk '{print $NF}')
if [ "$actual_version" != "$expected_version" ]; then
  echo "ESS $expected_version is required; found $actual_version" >&2
  exit 1
fi

generated=$(mktemp -d)
trap 'rm -rf "$generated"' EXIT

$ess_bin validate --path ess/system
$ess_bin build compile --path ess/build.yaml --out "$generated/build.json"
$ess_bin build graph --path ess/build.yaml --out "$generated/build.mmd"
$ess_bin project buildkit --ir "$generated/build.json" --out "$generated/buildkit"

awk '
  /<!-- ess-build-graph:begin -->/ { in_block = 1; next }
  /<!-- ess-build-graph:end -->/ { in_block = 0; next }
  in_block && /^```mermaid$/ { in_fence = 1; next }
  in_block && in_fence && /^```$/ { in_fence = 0; next }
  in_block && in_fence { print }
' docs/ess-deployment-model.md > "$generated/documented-build.mmd"

cmp generated/ess/build.json "$generated/build.json"
cmp generated/ess/build.mmd "$generated/build.mmd"
cmp generated/ess/build.mmd "$generated/documented-build.mmd"
cmp generated/ess/build.json "$generated/buildkit/ess-build-ir.json"
cmp Dockerfile.ess "$generated/buildkit/Dockerfile.ess"
cmp docker-bake.hcl "$generated/buildkit/docker-bake.hcl"
