#!/usr/bin/env bash
set -euo pipefail
# Validate the exact selected/reused composition before it becomes a baseline.
manifest=${1:?}
ctl=${2:?}
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
chart_version=$(jq -er '.provenance.chart.version' "$manifest")
chart_digest=$(jq -er '.artifacts.chart' "$manifest")
server_version=$(jq -er '.provenance.server.version' "$manifest")
server_digest=$(jq -er '.artifacts.devcenter' "$manifest")
connectors_version=$(jq -er '.provenance.connectors.version' "$manifest")
connectors_digest=$(jq -er '.artifacts.devcenter_connectors' "$manifest")
reference="ghcr.io/${GITHUB_REPOSITORY_OWNER:?}/devcenter"
output=$(helm pull "oci://ghcr.io/${GITHUB_REPOSITORY_OWNER}/charts/devcenter" --version "$chart_version" --destination "$scratch" 2>&1)
test "$(awk '$1 == "Digest:" {print $2}' <<<"$output")" = "$chart_digest"
chart="$scratch/devcenter-${chart_version}.tgz"
jq -n --arg reference "$reference" --arg server "$server_digest" --arg connectors "$connectors_digest" \
  '{global:{tenantId:"release-candidate", publicOrigin:"https://devcenter.example.invalid"}, devcenter:{image:{repository:$reference,digest:$server}, database:{existingSecret:"release-candidate-database"}}, components:{connectors:{enabled:true,image:{repository:$reference,digest:$connectors}}}, networkPolicy:{enabled:false}}' > "$scratch/values.json"
printf '%s\n' \
  'schema = 1' '[chart]' \
  "reference = \"$chart\"" "version = \"$chart_version\"" "digest = \"$chart_digest\"" \
  '[images.devcenter]' "reference = \"$reference\"" "version = \"$server_version\"" "digest = \"$server_digest\"" \
  '[images.connectors]' "reference = \"$reference\"" "version = \"connectors-$connectors_version\"" "digest = \"$connectors_digest\"" \
  > "$scratch/deployment.lock.toml"
helm lint "$chart" --values "$scratch/values.json"
"$ctl" deployment validate --release devcenter --namespace release-candidate \
  --chart "$chart" --version "$chart_version" --values "$scratch/values.json" \
  --lock "$scratch/deployment.lock.toml" --require-component devcenter --require-component connectors
