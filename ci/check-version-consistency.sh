#!/usr/bin/env bash
set -euo pipefail

expected_version=${1:-}
cargo_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
chart_version=$(sed -n 's/^version: //p' deploy/charts/devcenter/Chart.yaml)
chart_app_version=$(sed -n 's/^appVersion: "\([^"]*\)"/\1/p' deploy/charts/devcenter/Chart.yaml)
frontend_version=$(sed -n 's/^  "version": "\([^"]*\)",/\1/p' frontend/package.json)
openapi_version=$(sed -n 's/^    "version": "\([^"]*\)",/\1/p' openapi.json)

for named_version in \
  "chart version:$chart_version" \
  "chart appVersion:$chart_app_version" \
  "frontend package:$frontend_version" \
  "OpenAPI:$openapi_version"
do
  name=${named_version%%:*}
  version=${named_version#*:}
  if [[ -z "$version" || "$version" != "$cargo_version" ]]; then
    echo "$name version '$version' does not match Cargo version '$cargo_version'" >&2
    exit 1
  fi
done

if [[ -n "$expected_version" && "$cargo_version" != "$expected_version" ]]; then
  echo "Cargo version '$cargo_version' does not match release tag '$expected_version'" >&2
  exit 1
fi
