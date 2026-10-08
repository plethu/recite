#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"
if [[ -n "${RECITE_HEAD_REF:-}" ]]; then
  scripts/check-maintainability.sh
  scripts/check-lint-suppressions.sh
else
  # Snapshot working bytes, including untracked source, without changing the
  # contributor's staging area or creating a commit, ref or second checkout.
  temporary="$(mktemp -d)"
  trap 'rm -rf "$temporary"' EXIT
  git ls-files --cached -z >"$temporary/tracked"
  export GIT_INDEX_FILE="$temporary/index"
  git read-tree HEAD
  # Newly force-staged ignored files are source too. Retain their membership,
  # but read current bytes rather than staged blobs or cached status flags.
  git update-index --add --remove -z --stdin <"$temporary/tracked"
  git add -A -- .
  head_tree="$(git write-tree)"
  base="${RECITE_BASE_REF:-origin/main}"
  if [[ ! "$base" =~ ^0{40}$ ]]; then
    base="$(git merge-base "$base" HEAD)"
  fi
  scripts/check-maintainability.sh "$base" "$head_tree"
  scripts/check-lint-suppressions.sh "$base" "$head_tree"
fi
