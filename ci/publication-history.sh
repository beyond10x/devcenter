#!/usr/bin/env bash
set -euo pipefail
# Only completed GitHub releases are baselines. Workflow artifacts expire and
# partial registry pushes carry no authority to advance a source baseline.
destination=${1:?}
requested=${2:-}
scratch=$(mktemp -d)
trap 'rm -rf -- "$scratch"' EXIT
gh api --paginate "repos/${GITHUB_REPOSITORY:?}/releases?per_page=100" | \
  jq -s 'add | map(select(.draft == false and .prerelease == false) | select(.tag_name | test("^([0-9]+\\.[0-9]+\\.[0-9]+|publication-.+)$"))) | sort_by(.published_at) | reverse' > "$scratch/releases.json"
if jq -e 'length > 0' "$scratch/releases.json" >/dev/null; then
  jq -e '.[0].assets | any(.name == "release-manifest.json")' "$scratch/releases.json" >/dev/null || {
    echo 'latest completed publication has no manifest; refusing incomplete history' >&2
    exit 1
  }
fi
# The latest full manifest already preserves every artifact's own successful
# baseline. Also retrieve an existing requested identifier to enforce immutability.
# Listing is paginated; downloading every historical asset would grow linearly.
jq --arg requested "$requested" '[to_entries[] | select(.key == 0 or .value.tag_name == $requested or .value.tag_name == ("publication-" + $requested)) | .value]' \
  "$scratch/releases.json" > "$scratch/relevant.json"
jq -e 'all(.[]; .assets | any(.name == "release-manifest.json"))' "$scratch/relevant.json" >/dev/null || {
  echo 'requested publication has no durable manifest; choose a new identifier' >&2
  exit 1
}
index=0
while IFS= read -r tag; do
  mkdir "$scratch/$index"
  gh release download "$tag" --repo "$GITHUB_REPOSITORY" --pattern release-manifest.json --dir "$scratch/$index"
  jq -e 'type == "object"' "$scratch/$index/release-manifest.json" >/dev/null
  index=$((index + 1))
done < <(jq -r '.[].tag_name' "$scratch/relevant.json")
if [ "$index" -eq 0 ]; then
  printf '[]\n' > "$destination"
else
  # Numeric iteration preserves published_at ordering even beyond ten releases.
  files=()
  for ((i=0; i<index; i++)); do files+=("$scratch/$i/release-manifest.json"); done
  jq -s '.' "${files[@]}" > "$destination"
fi
