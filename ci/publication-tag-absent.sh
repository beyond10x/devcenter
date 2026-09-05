#!/usr/bin/env bash
set -euo pipefail
# Never replace a pushed tag, including a partial failed publication. Such a
# candidate needs a new identifier; successful retries are skipped by the planner.
if output=$(docker buildx imagetools inspect "${1:?}" 2>&1); then
  echo "refusing to overwrite published artifact ${1}" >&2
  exit 1
fi
case "$output" in
  *'manifest unknown'*|*"$1: not found"*) ;;
  *) printf '%s\n' "$output" >&2; exit 1 ;;
esac
