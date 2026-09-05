#!/usr/bin/env bash
set -euo pipefail

chart=deploy/charts/devcenter
values="$chart/ci/test-values.yaml"

rendered=$(mktemp)
missing_database_error=$(mktemp)
invalid_docs_error=$(mktemp)
invalid_identity_cli_error=$(mktemp)
invalid_connector_client_error=$(mktemp)
invalid_kubernetes_access_error=$(mktemp)
invalid_identity_provider_error=$(mktemp)
invalid_agentide_workspace_error=$(mktemp)
invalid_workflow_error=$(mktemp)
trap 'rm -f "$rendered" "$missing_database_error" "$invalid_docs_error" "$invalid_identity_cli_error" "$invalid_connector_client_error" "$invalid_kubernetes_access_error" "$invalid_identity_provider_error" "$invalid_agentide_workspace_error" "$invalid_workflow_error"' EXIT

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

component_config_checksum() {
  component=$1
  shift
  helm template devcenter "$chart" \
    --namespace devcenter \
    --values "$values" \
    "$@" \
    | awk -v expected="devcenter-$component" '
        /^kind: Deployment$/ { deployment = 1; name = ""; next }
        deployment && name == "" && /^  name: / { name = $2; next }
        deployment && name == expected && /checksum\/component-config:/ {
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

default_component_checksum=$(component_config_checksum connectors)
alternate_component_checksum=$(
  component_config_checksum connectors \
    --set-string 'components.connectors.configFiles.services\.yaml=services: []'
)

test -n "$default_component_checksum"
test -n "$alternate_component_checksum"
test "$default_component_checksum" != "$alternate_component_checksum"

helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  > "$rendered"

# The admitted HTTPS origin and Service use 443, while the process and network policy
# use the unprivileged listener. Substrate requires the CA bundle to be a regular file.
grep -Fq 'value: "https://devcenter-connectors.devcenter.svc.cluster.local:443"' "$rendered"
grep -Fq 'value: "0.0.0.0:8443"' "$rendered"
grep -Fq 'name: git-fetch-tls, port: 443, targetPort: git-fetch-tls' "$rendered"
grep -Fq 'name: git-fetch-tls, containerPort: 8443' "$rendered"
grep -Fq 'name: connectors-git-fetch-ca, mountPath: /etc/substrate/git-fetch/ca.crt, subPath: ca.crt, readOnly: true' "$rendered"
if helm template devcenter "$chart" --namespace devcenter --values "$values" \
  --set connectorsGitFetch.port=8443 >/dev/null 2>"$invalid_workflow_error"; then
  echo "chart unexpectedly admitted a Git fetch origin outside HTTPS port 443" >&2
  exit 1
fi
grep -q '443' "$invalid_workflow_error"

# Quota-backed Git workspaces keep state on its existing claim and use an explicitly
# selected filesystem. The default daemon must retain its unprivileged posture.
substrate_render() {
  helm template devcenter "$chart" --namespace devcenter --values "$values" "$@" \
    | awk '/^# Source: .*\/substrate.yaml$/ { selected = 1; next } /^---$/ { selected = 0 } selected'
}
default_substrate=$(substrate_render)
if grep -Eq 'add:.*SYS_ADMIN|--project-quota-ids|name: workspace-data|substrate-daemon-quota' <<<"$default_substrate"; then
  echo "default Substrate unexpectedly enables project quota authority" >&2
  exit 1
fi
quota_substrate=$(substrate_render \
  --set substrate.workspaceStorage.existingClaim=quota-workspaces \
  --set substrate.workspaceStorage.projectQuotas.enabled=true \
  --set substrate.workspaceStorage.projectQuotas.idsStart=200000 \
  --set substrate.workspaceStorage.projectQuotas.idsEnd=204095)
grep -Fq -- '- --project-quota-ids' <<<"$quota_substrate"
grep -Fq 'command: ["/usr/local/bin/substrate-daemon-quota"]' <<<"$quota_substrate"
grep -Fq -- '- "200000-204095"' <<<"$quota_substrate"
grep -Fq 'claimName: "quota-workspaces"' <<<"$quota_substrate"
test "$(grep -Fc 'name: workspace-data, mountPath: /var/lib/substrate/workspaces' <<<"$quota_substrate")" -eq 2
grep -Fq 'capabilities: {drop: ["ALL"], add: ["SYS_ADMIN"]}' <<<"$quota_substrate"
grep -Fq 'allowPrivilegeEscalation: true' <<<"$quota_substrate"
grep -Fq 'runAsNonRoot: true' <<<"$quota_substrate"
grep -Fq 'readOnlyRootFilesystem: true' <<<"$quota_substrate"
if grep -Eq 'privileged: true|hostPath:|hostPID: true|hostNetwork: true|SYS_RESOURCE' <<<"$quota_substrate"; then
  echo "project quota configuration enables unrelated host authority" >&2
  exit 1
fi
for invalid_quota in missing-claim reversed-range short-range; do
  quota_args=(--set substrate.workspaceStorage.projectQuotas.enabled=true)
  case "$invalid_quota" in
    missing-claim) ;;
    reversed-range) quota_args+=(--set substrate.workspaceStorage.existingClaim=quota-workspaces --set substrate.workspaceStorage.projectQuotas.idsStart=300000 --set substrate.workspaceStorage.projectQuotas.idsEnd=200000) ;;
    short-range) quota_args+=(--set substrate.workspaceStorage.existingClaim=quota-workspaces --set substrate.workspaceStorage.projectQuotas.idsStart=200000 --set substrate.workspaceStorage.projectQuotas.idsEnd=200126) ;;
  esac
  if substrate_render "${quota_args[@]}" >/dev/null 2>"$invalid_workflow_error"; then
    echo "chart unexpectedly admitted project quota configuration: $invalid_quota" >&2
    exit 1
  fi
  grep -q 'project quota' "$invalid_workflow_error"
done

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
grep -q 'name: WORKFLOW_IDENTITY_ORIGIN' "$rendered"
grep -A1 'name: WORKFLOW_IDENTITY_AUDIENCE' "$rendered" | grep -q 'value: "urn:b10x:workflow"'
grep -q 'name: AGENT_PLATFORM_IDENTITY_ORIGIN' "$rendered"
grep -q 'name: AGENT_PLATFORM_CONNECTORS_API_BASE' "$rendered"
grep -q 'name: AGENT_PLATFORM_WORKSPACE_ORIGIN' "$rendered"
grep -q 'name: AGENT_PLATFORM_STATE_PATH' "$rendered"
grep -q 'value: "/var/lib/agent-platform/state.json"' "$rendered"
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
grep -q 'DEV_CENTER_AGENTIDE_WORKSPACE_ENABLED: "true"' "$rendered"
grep -q 'DEV_CENTER_WORKFLOW_ORIGIN:' "$rendered"
grep -A1 'name: CONNECTORS_GIT_FETCH_ORIGIN' "$rendered" | grep -q 'https://devcenter-connectors.devcenter.svc.cluster.local:443'
grep -A1 'name: CONNECTORS_GIT_FETCH_TLS_LISTEN' "$rendered" | grep -q '0.0.0.0:8443'
grep -A1 'name: CONNECTORS_GIT_FETCH_TLS_CERTIFICATE_FILE' "$rendered" | grep -q '/etc/connectors/git-fetch/tls.crt'
grep -A1 'name: CONNECTORS_GIT_FETCH_TLS_PRIVATE_KEY_FILE' "$rendered" | grep -q '/etc/connectors/git-fetch/tls.key'

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set components.agent-platform.enabled=false \
  >/dev/null 2>"$invalid_agentide_workspace_error"
then
  echo "chart unexpectedly enabled the AgentIDE workspace without Agent Platform" >&2
  exit 1
fi
grep -q "the AgentIDE workspace requires components.agent-platform.enabled" "$invalid_agentide_workspace_error"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set devcenter.features.agentideWorkspace.enabled=false \
  --set components.identity.enabled=false \
  --set components.secrets.enabled=false \
  >/dev/null 2>"$invalid_workflow_error"
then
  echo "chart unexpectedly enabled Workflow without Identity" >&2
  exit 1
fi
grep -q "the Workflow library requires components.identity.enabled" "$invalid_workflow_error"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set 'devcenter.identity.providers[0].id=default' \
  --set 'devcenter.identity.providers[0].displayName=GitLab' \
  >/dev/null 2>"$invalid_identity_provider_error"
then
  echo "chart unexpectedly accepted an invalid Identity provider wire field" >&2
  exit 1
fi
grep -q 'displayName' "$invalid_identity_provider_error"

helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set connectorsKubernetesAccess.enabled=true \
  --set 'connectorsKubernetesAccess.namespaces[0]=devcenter' \
  --set 'connectorsKubernetesAccess.namespaces[1]=latest' \
  --set 'connectorsKubernetesAccess.apiServerCidrs[0]=172.20.0.1/32' \
  > "$rendered"

test "$(grep -c '^kind: Role$' "$rendered")" -eq 2
test "$(grep -c '^kind: RoleBinding$' "$rendered")" -eq 2
grep -q '^  namespace: devcenter$' "$rendered"
grep -q '^  namespace: latest$' "$rendered"
grep -q 'resources: \["deployments"\]' "$rendered"
grep -q 'resources: \["pods", "events"\]' "$rendered"
grep -q 'resources: \["pods/log"\]' "$rendered"
grep -q 'cidr: "172.20.0.1/32"' "$rendered"
if grep -A12 '^kind: Role$' "$rendered" | grep -Eq 'secrets|create|delete|patch|update'; then
  echo "namespace read Role unexpectedly grants sensitive or mutating authority" >&2
  exit 1
fi

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set connectorsKubernetesAccess.enabled=true \
  --set 'connectorsKubernetesAccess.apiServerCidrs[0]=172.20.0.1/32' \
  >/dev/null 2>"$invalid_kubernetes_access_error"
then
  echo "chart unexpectedly rendered Kubernetes access without exact namespaces" >&2
  exit 1
fi
grep -q "connectorsKubernetesAccess.namespaces must contain at least one exact namespace" "$invalid_kubernetes_access_error"

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
  --set ingress.identityCliRoutes.enabled=true \
  --set ingress.connectorClientApi.enabled=true \
  --set ingress.connectorAdminRoutes.enabled=true \
  --set ingress.connectorDocs.enabled=true \
  --set networkPolicy.enabled=false \
  --set connectorsGitFetch.enabled=false \
  > "$rendered"

grep -q 'path: /api/connectors/v1/connect-sessions' "$rendered"
grep -q 'path: /api/connectors/v1/oauth/gitlab/callback' "$rendered"
grep -A1 'path: /.well-known/identity-cli-login' "$rendered" | grep -q 'pathType: Exact'
grep -A1 'path: /v1/access-token' "$rendered" | grep -q 'pathType: Exact'
grep -A1 'path: /api/connectors/v1$' "$rendered" | grep -q 'pathType: Prefix'
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

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.identityCliRoutes.enabled=true \
  >/dev/null 2>"$invalid_identity_cli_error"
then
  echo "chart unexpectedly exposed Identity CLI routes without ingress" >&2
  exit 1
fi
grep -q "ingress.identityCliRoutes.enabled requires ingress.enabled" "$invalid_identity_cli_error"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.enabled=true \
  --set ingress.host=devcenter.example.invalid \
  --set ingress.tls.enabled=false \
  --set ingress.identityCliRoutes.enabled=true \
  --set components.identity.enabled=false \
  --set components.secrets.enabled=false \
  --set networkPolicy.enabled=false \
  --set connectorsGitFetch.enabled=false \
  >/dev/null 2>"$invalid_identity_cli_error"
then
  echo "chart unexpectedly exposed Identity CLI routes without Identity" >&2
  exit 1
fi
grep -q "ingress.identityCliRoutes.enabled requires components.identity.enabled" "$invalid_identity_cli_error"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.connectorClientApi.enabled=true \
  >/dev/null 2>"$invalid_connector_client_error"
then
  echo "chart unexpectedly exposed the Connector client API without ingress" >&2
  exit 1
fi
grep -q "ingress.connectorClientApi.enabled requires ingress.enabled" "$invalid_connector_client_error"

if helm template devcenter "$chart" \
  --namespace devcenter \
  --values "$values" \
  --set ingress.enabled=true \
  --set ingress.host=devcenter.example.invalid \
  --set ingress.tls.enabled=false \
  --set ingress.connectorClientApi.enabled=true \
  --set components.connectors.enabled=false \
  --set components.secrets.enabled=false \
  --set networkPolicy.enabled=false \
  --set connectorsGitFetch.enabled=false \
  >/dev/null 2>"$invalid_connector_client_error"
then
  echo "chart unexpectedly exposed the Connector client API without Connectors" >&2
  exit 1
fi
grep -q "ingress.connectorClientApi.enabled requires components.connectors.enabled" "$invalid_connector_client_error"
