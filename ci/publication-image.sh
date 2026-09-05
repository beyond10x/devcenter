#!/usr/bin/env bash
set -euo pipefail
# Registry transport coordinates preserve the existing digest consumers.
owner=${GITHUB_REPOSITORY_OWNER:?}
case "${1:?}" in
  server) printf 'ghcr.io/%s/devcenter:%s\n' "$owner" "${2:?}" ;;
  connectors) printf 'ghcr.io/%s/devcenter:connectors-%s\n' "$owner" "${2:?}" ;;
  deployment-cli) printf 'ghcr.io/%s/devcenterctl:%s\n' "$owner" "${2:?}" ;;
  *) echo 'unknown image output' >&2; exit 2 ;;
esac
