#!/usr/bin/env bash
set -euo pipefail

chart=deploy/charts/devcenter
values="$chart/ci/test-values.yaml"

rendered=$(mktemp)
missing_database_error=$(mktemp)
trap 'rm -f "$rendered" "$missing_database_error"' EXIT

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

helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  > "$rendered"

for deployment_name in \
  devcenter \
  devcenter-aep-service \
  devcenter-agent-platform \
  devcenter-connectors \
  devcenter-identity \
  devcenter-llmgw \
  devcenter-secrets \
  devcenter-workflow
do
  awk -v expected="$deployment_name" '
    /^kind: Deployment$/ { deployment = 1; name = ""; ready = 0; live = 0; resources = 0; next }
    deployment && name == "" && /^  name: / { name = $2; next }
    deployment && /readinessProbe:/ { ready = 1 }
    deployment && /livenessProbe:/ { live = 1 }
    deployment && /resources:/ { resources = 1 }
    /^---$/ {
      if (deployment && name == expected && ready && live && resources) found = 1
      deployment = 0
    }
    END { exit(found ? 0 : 1) }
  ' "$rendered"
done

awk '
  /^kind: StatefulSet$/ { statefulset = 1; name = ""; ready = 0; live = 0; resources = 0; next }
  statefulset && name == "" && /^  name: / { name = $2; next }
  statefulset && /readinessProbe:/ { ready = 1 }
  statefulset && /livenessProbe:/ { live = 1 }
  statefulset && /resources:/ { resources = 1 }
  /^---$/ {
    if (statefulset && name == "devcenter-substrate" && ready && live && resources) found = 1
    statefulset = 0
  }
  END {
    if (statefulset && name == "devcenter-substrate" && ready && live && resources) found = 1
    exit(found ? 0 : 1)
  }
' "$rendered"

grep -q 'name: SUBSTRATE_TLS_LISTEN' "$rendered"
grep -q 'name: WORKSPACE_SUBSTRATE_ORIGIN' "$rendered"
grep -q 'name: volume-permissions' "$rendered"
grep -q 'chown 65532:65532 /var/lib/substrate /var/run/substrate' "$rendered"
grep -q 'chmod 0700 /var/lib/substrate /var/run/substrate' "$rendered"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set devcenter.database.existingSecret= \
  >/dev/null 2>"$missing_database_error"
then
  echo "chart unexpectedly rendered without devcenter.database.existingSecret" >&2
  exit 1
fi
grep -q "devcenter.database.existingSecret is required" "$missing_database_error"
