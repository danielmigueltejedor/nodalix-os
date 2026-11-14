#!/usr/bin/env bash
# Fails unless the release notes of a version are written by hand and complete.
#
# Usage:
#   scripts/check-release-notes.sh             the version in VERSION
#   scripts/check-release-notes.sh X.Y.Z       that version's notes (with or without a v)
#   scripts/check-release-notes.sh --changed BASE
#       what a pull request changed since commit BASE: the new version when the change bumps
#       VERSION, and every notes file it adds or edits. This is the check the "Release notes are
#       written" job runs.
#
# The same interface as every Nodalia app's script; Nodalix's own rules live in
# tools/release_notes.py (validate), which release.yml runs again before it publishes.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
validate() { (cd "$root" && python3 tools/release_notes.py validate --version "$1"); }

if [[ "${1:-}" == "--changed" ]]; then
  base="${2:?--changed needs the base commit}"
  head_version="$(tr -d '[:space:]' <"$root/VERSION")"
  base_version="$(git -C "$root" show "$base:VERSION" 2>/dev/null | tr -d '[:space:]' || true)"
  versions=()
  if [[ -n "$head_version" && "$head_version" != "$base_version" ]]; then
    versions+=("$head_version")
  fi
  while IFS= read -r changed; do
    [[ -n "$changed" ]] && versions+=("$changed")
  # Whole-tree diff, so a notes file that only moved here counts as renamed, not added.
  done < <(git -C "$root" diff --name-only -M --diff-filter=AM "$base" HEAD \
    | sed -n 's#^docs/release-notes/v\{0,1\}\([0-9][^/]*\)\.md$#\1#p')
  if [[ "${#versions[@]}" -eq 0 ]]; then
    echo "✓ No version bump and no notes changed: nothing to check."
    exit 0
  fi
  status=0
  for version in $(printf '%s\n' "${versions[@]}" | sort -u); do
    validate "$version" || status=1
  done
  exit "$status"
fi

validate "${1:-$(tr -d '[:space:]' <"$root/VERSION")}"
