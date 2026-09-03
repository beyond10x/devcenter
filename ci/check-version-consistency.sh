#!/usr/bin/env bash
set -euo pipefail

expected_version=${1:-}
cargo_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
chart_version=$(sed -n 's/^version: //p' deploy/charts/devcenter/Chart.yaml)
chart_app_version=$(sed -n 's/^appVersion: "\([^"]*\)"/\1/p' deploy/charts/devcenter/Chart.yaml)
frontend_version=$(sed -n 's/^  "version": "\([^"]*\)",/\1/p' frontend/package.json)
openapi_version=$(sed -n 's/^    "version": "\([^"]*\)",/\1/p' openapi.json)
connectors_version=$(sed -n 's/^version = "\([^"]*\)"/\1/p' crates/devcenter-connectors/Cargo.toml | head -1)
rust_version=$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' Cargo.toml | head -1)
connectors_rust_version=$(sed -n 's/^rust-version = "\([^"]*\)"/\1/p' crates/devcenter-connectors/Cargo.toml | head -1)
toolchain_version=$(sed -n 's/^channel = "\([^"]*\)"/\1/p' rust-toolchain.toml | head -1)
mapfile -t image_rust_versions < <(sed -n 's/^FROM rust:\([^-@]*\)-bookworm@.* AS .*builder$/\1/p' Dockerfile)

normalize_rust_version() {
  case "$1" in
    *.*.*) printf '%s\n' "$1" ;;
    *.*) printf '%s.0\n' "$1" ;;
    *) return 1 ;;
  esac
}

for named_version in \
  "chart version:$chart_version" \
  "chart appVersion:$chart_app_version" \
  "frontend package:$frontend_version" \
  "OpenAPI:$openapi_version" \
  "composed Connectors:$connectors_version"
do
  name=${named_version%%:*}
  version=${named_version#*:}
  if [[ -z "$version" || "$version" != "$cargo_version" ]]; then
    echo "$name version '$version' does not match Cargo version '$cargo_version'" >&2
    exit 1
  fi
done

normalized_rust_version=$(normalize_rust_version "$rust_version")
if [[ "$connectors_rust_version" != "$rust_version" || "$toolchain_version" != "$normalized_rust_version" ]]; then
  echo "Rust versions are inconsistent: workspace=$rust_version composed=$connectors_rust_version toolchain=$toolchain_version" >&2
  exit 1
fi
if [[ ${#image_rust_versions[@]} -ne 2 ]]; then
  echo "expected exactly two pinned Rust builder images, found ${#image_rust_versions[@]}" >&2
  exit 1
fi
for image_rust_version in "${image_rust_versions[@]}"; do
  if [[ "$image_rust_version" != "$normalized_rust_version" ]]; then
    echo "builder image Rust '$image_rust_version' does not match workspace Rust '$normalized_rust_version'" >&2
    exit 1
  fi
done

if [[ -n "$expected_version" && "$cargo_version" != "$expected_version" ]]; then
  echo "Cargo version '$cargo_version' does not match release tag '$expected_version'" >&2
  exit 1
fi
