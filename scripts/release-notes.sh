#!/usr/bin/env bash
# Affiche la section `## [<version>]` de CHANGELOG.md (notes de la release).
#   usage : scripts/release-notes.sh 0.1.0
set -euo pipefail
version=${1:?usage: scripts/release-notes.sh <version>}
cd "$(dirname "$0")/.."
notes=$(awk -v v="## [$version]" '
  index($0, v) == 1 { on = 1; next }
  on && /^## \[/ { exit }
  on { print }
' CHANGELOG.md)
if [ -z "${notes//[[:space:]]/}" ]; then
  echo "no CHANGELOG.md section for $version" >&2
  exit 1
fi
printf '%s\n' "$notes"
