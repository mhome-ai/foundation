#!/usr/bin/env bash
# Release tags must refer to main or a coordinator-owned publication branch.
set -euo pipefail
revision="${1:?expected the full release commit SHA}"
[[ "$revision" =~ ^[0-9a-f]{40}$ ]] || { echo "Invalid release commit SHA" >&2; exit 1; }
git fetch --no-tags --prune origin \
  '+refs/heads/main:refs/remotes/origin/main' \
  '+refs/heads/repo-update/*:refs/remotes/origin/repo-update/*'
sources="$(git for-each-ref --contains="$revision" --format='%(refname)' \
  refs/remotes/origin/main refs/remotes/origin/repo-update/)"
[[ -n "$sources" ]] || { echo "Release commit is not on main or a repo-update publication branch" >&2; exit 1; }
printf 'Verified release source %s\n%s\n' "$revision" "$sources"
