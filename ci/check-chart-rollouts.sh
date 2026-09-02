#!/usr/bin/env bash
set -euo pipefail

chart=deploy/charts/devcenter
values="$chart/ci/test-values.yaml"

rendered=$(mktemp)
missing_database_error=$(mktemp)
invalid_docs_error=$(mktemp)
trap 'rm -f "$rendered" "$missing_database_error" "$invalid_docs_error"' EXIT

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
grep -q 'path: /api/connectors/v1/readyz' "$rendered"
grep -q 'path: /api/connectors/v1/livez' "$rendered"
grep -q 'name: volume-permissions' "$rendered"
grep -q 'chown 0:0 /var/run/substrate-tls' "$rendered"
grep -q 'chmod 0700 /var/lib/substrate /var/run/substrate /var/run/substrate-tls' "$rendered"
grep -q 'rm -f /var/run/substrate-tls/tls.crt /var/run/substrate-tls/tls.key' "$rendered"
grep -q 'cp /var/run/substrate-tls-source/tls.crt /var/run/substrate-tls/tls.crt' "$rendered"
grep -q 'cp /var/run/substrate-tls-source/tls.key /var/run/substrate-tls/tls.key' "$rendered"
grep -q 'chmod 0600 /var/run/substrate-tls/tls.key' "$rendered"
grep -q 'chown 65532:65532 /var/run/substrate-tls/tls.crt /var/run/substrate-tls/tls.key' "$rendered"
grep -q 'chown 65532:65532 /var/lib/substrate /var/run/substrate /var/run/substrate-tls' "$rendered"
grep -q 'name: tls-source' "$rendered"
grep -q 'DEV_CENTER_CONNECTORS_DOCS_AVAILABLE: "false"' "$rendered"

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

helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.enabled=true \
  --set ingress.host=devcenter.example.invalid \
  --set ingress.tls.enabled=false \
  --set ingress.connectorSetupRoutes.enabled=true \
  --set ingress.connectorAdminRoutes.enabled=true \
  --set ingress.connectorDocs.enabled=true \
  --set networkPolicy.enabled=false \
  > "$rendered"

grep -q 'path: /api/connectors/v1/connect-sessions' "$rendered"
grep -q 'path: /api/connectors/v1/oauth/gitlab/callback' "$rendered"
grep -q 'path: /api/connectors/v1/admin' "$rendered"
grep -q 'path: /api/connectors/v1/docs' "$rendered"
grep -q 'path: /api/connectors/v1/openapi.json' "$rendered"
grep -q 'DEV_CENTER_CONNECTORS_DOCS_AVAILABLE: "true"' "$rendered"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.connectorDocs.enabled=true \
  >/dev/null 2>"$invalid_docs_error"
then
  echo "chart unexpectedly exposed Connector docs without ingress" >&2
  exit 1
fi
grep -q "ingress.connectorDocs.enabled requires ingress.enabled" "$invalid_docs_error"
