#!/usr/bin/env bash
set -euo pipefail

chart=deploy/charts/devcenter
image_digest=sha256:1c59e2c3c818eaa0f0628f695b36e7c9e362d6b219b36a54a32df645cbd7e1af
image="postgres@$image_digest"
fixture=$(mktemp -d)
container=
executed=0
failed=0
cleanup() {
  if [[ -n "$container" ]]; then
    docker rm --force --volumes "$container" >/dev/null
  fi
  rm -rf "$fixture"
}
trap cleanup EXIT

# Public placeholder bytes only: the initializer copies files, not certificate contents.
mkdir "$fixture/tls-source"
chmod 0755 "$fixture" "$fixture/tls-source"
printf 'fixture certificate\n' > "$fixture/tls-source/tls.crt"
printf 'fixture private key\n' > "$fixture/tls-source/tls.key"
chmod 0444 "$fixture/tls-source/"*

for layout in shared separate; do
  render_args=()
  workspace_mount=()
  if [[ "$layout" == separate ]]; then
    render_args+=(--set substrate.workspaceStorage.existingClaim=quota-workspaces)
    workspace_mount+=(--tmpfs /var/lib/substrate/workspaces:rw,nosuid,nodev,noexec,mode=0700)
  fi
  manifest=$(helm template devcenter "$chart" --namespace devcenter \
    --values "$chart/ci/test-values.yaml" --show-only templates/substrate.yaml \
    --set substrate.volumePermissions.image.repository=postgres \
    --set "substrate.volumePermissions.image.digest=$image_digest" "${render_args[@]}")
  initializer=$(awk '
    /^        - name: volume-permissions$/ { selected = 1 }
    /^      containers:/ { selected = 0 }
    selected
  ' <<<"$manifest")
  command=$(awk '
    /^            - \|$/ { selected = 1; next }
    selected && /^              / { sub(/^              /, ""); print; next }
    selected { exit }
  ' <<<"$initializer")
  test -n "$command"
  grep -Fq "image: \"$image\"" <<<"$initializer"
  grep -Fxq '            - sh' <<<"$initializer"
  grep -Fxq '            - -ec' <<<"$initializer"
  grep -Fxq '            runAsUser: 0' <<<"$initializer"
  grep -Fxq '            runAsGroup: 0' <<<"$initializer"
  grep -Fxq '            allowPrivilegeEscalation: false' <<<"$initializer"
  grep -Fxq '            readOnlyRootFilesystem: true' <<<"$initializer"
  grep -Fxq '            capabilities: {drop: ["ALL"], add: ["CHOWN", "FOWNER"]}' <<<"$initializer"

  for initial_owner in 0 65532; do
    container=$(docker run --detach --rm \
      --name "devcenter-volume-permissions-${fixture##*/}-$layout-$initial_owner" \
      --network none --read-only --user 0:0 --workdir / \
      --cap-drop ALL --cap-add CHOWN --cap-add FOWNER \
      --security-opt no-new-privileges \
      --tmpfs /var/lib/substrate:rw,nosuid,nodev,noexec,mode=0700 \
      "${workspace_mount[@]}" \
      --tmpfs /var/run/substrate:rw,nosuid,nodev,noexec,mode=0700 \
      --tmpfs /var/run/substrate-tls:rw,nosuid,nodev,noexec,mode=0700 \
      --mount "type=bind,src=$fixture/tls-source,dst=/var/run/substrate-tls-source,readonly" \
      --entrypoint sleep "$image" 300)
    printf 'fixture: %s layout=%s initial-owner=%s\n' "$container" "$layout" "$initial_owner"
    docker exec "$container" sh -ec '
      test "$(id -u):$(id -g)" = 0:0
      for mask in CapPrm CapEff CapBnd; do
        test "$(awk -v key="$mask:" '\''$1 == key { print $2 }'\'' /proc/self/status)" = 0000000000000009
      done
      for mask in CapInh CapAmb; do
        test "$(awk -v key="$mask:" '\''$1 == key { print $2 }'\'' /proc/self/status)" = 0000000000000000
      done
      test "$(awk '\''$1 == "NoNewPrivs:" { print $2 }'\'' /proc/self/status)" = 1
      if touch /root-filesystem-must-be-read-only 2>/dev/null; then exit 1; fi
      if touch /var/run/substrate-tls-source/must-be-read-only 2>/dev/null; then exit 1; fi
      umask 077
      mkdir -p /var/lib/substrate/workspaces/.baseline
      printf "durable state\n" > /var/lib/substrate/state.sqlite
      printf "workspace data\n" > /var/lib/substrate/workspaces/data
      printf "hidden baseline\n" > /var/lib/substrate/workspaces/.baseline/data
      printf "old certificate\n" > /var/run/substrate-tls/tls.crt
      printf "old private key\n" > /var/run/substrate-tls/tls.key
      chown 65532:65532 /var/lib/substrate/state.sqlite \
        /var/lib/substrate/workspaces/data /var/lib/substrate/workspaces/.baseline/data \
        /var/lib/substrate/workspaces/.baseline /var/lib/substrate/workspaces \
        /var/run/substrate-tls/tls.crt /var/run/substrate-tls/tls.key \
        /var/run/substrate /var/run/substrate-tls
      chown "$1:$1" /var/lib/substrate
    ' sh "$initial_owner"

    # Both runs use the unmodified rendered command. The second starts with the private
    # ownership left by the first, just as a replacement pod revisits existing volumes.
    for run in 1 2; do
      executed=$((executed + 1))
      if docker exec "$container" sh -ec "$command" &&
        docker exec --user 65532:65532 "$container" sh -ec '
          for directory in /var/lib/substrate /var/lib/substrate/workspaces \
            /var/lib/substrate/workspaces/.baseline /var/run/substrate /var/run/substrate-tls; do
            test "$(stat -c %u:%g:%a "$directory")" = 65532:65532:700
          done
          for file in /var/lib/substrate/state.sqlite /var/lib/substrate/workspaces/data \
            /var/lib/substrate/workspaces/.baseline/data /var/run/substrate-tls/tls.key; do
            test "$(stat -c %u:%g:%a "$file")" = 65532:65532:600
          done
          test "$(stat -c %u:%g:%a /var/run/substrate-tls/tls.crt)" = 65532:65532:644
          test "$(cat /var/lib/substrate/state.sqlite)" = "durable state"
          test "$(cat /var/lib/substrate/workspaces/data)" = "workspace data"
          test "$(cat /var/lib/substrate/workspaces/.baseline/data)" = "hidden baseline"
          cmp /var/run/substrate-tls-source/tls.crt /var/run/substrate-tls/tls.crt
          cmp /var/run/substrate-tls-source/tls.key /var/run/substrate-tls/tls.key
        '
      then
        printf 'PASS: %s initial-owner=%s run=%s\n' "$layout" "$initial_owner" "$run"
      else
        failed=$((failed + 1))
        printf 'FAIL: %s initial-owner=%s run=%s\n' "$layout" "$initial_owner" "$run" >&2
      fi
    done
    docker rm --force --volumes "$container" >/dev/null
    container=
  done
done

printf 'volume permissions: executed %s, passed %s, failed %s\n' "$executed" "$((executed - failed))" "$failed"
test "$failed" -eq 0
