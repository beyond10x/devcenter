#!/usr/bin/env bash
set -euo pipefail

chart=deploy/charts/devcenter
values="$chart/ci/test-values.yaml"

grants_checksum() {
  helm template devcenter "$chart" \
    --namespace devcenter \
    --values "$values" \
    "$@" \
    | awk '
        /^kind: Deployment$/ { deployment = 1; name = ""; next }
        deployment && name == "" && /^  name: / { name = $2; next }
        deployment && name == "devcenter-secrets" && /checksum\/secrets-workloads:/ {
          gsub(/"/, "", $2)
          print $2
          exit
        }
        /^---$/ { deployment = 0; name = "" }
      '
}

default_checksum=$(grants_checksum)
alternate_checksum=$(grants_checksum --set components.connectors.serviceAccountName=alternate-connectors)

test -n "$default_checksum"
test -n "$alternate_checksum"
test "$default_checksum" != "$alternate_checksum"
