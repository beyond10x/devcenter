#!/usr/bin/env bash
set -euo pipefail
# CI adapter only: Rust owns gate and publication decisions.
if [[ -n "${DEVCENTERCTL:-}" ]]; then
  exec "$DEVCENTERCTL" release impact "$@"
fi
exec cargo run --quiet --locked -p devcenterctl -- release impact "$@"
