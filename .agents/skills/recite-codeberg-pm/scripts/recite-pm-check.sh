#!/usr/bin/env bash
set -euo pipefail

if ! command -v tea >/dev/null 2>&1; then
  echo "tea not installed; install and authenticate tea for Codeberg before running Recite PM checks" >&2
  exit 2
fi

echo "== git remote =="
git remote -v

echo
echo "== tea version =="
tea --version

echo
echo "== milestones =="
tea milestones list

echo
echo "== labels =="
tea labels list

echo
echo "== open issues, first 100 =="
tea issues list --state open --limit 100

echo
echo "== status labels defined =="
status_labels="$(tea labels list | grep -E 'status/(ready|design-needed|in-progress|review|blocked)' || true)"
if [[ -n "$status_labels" ]]; then
  printf '%s\n' "$status_labels"
else
  echo "(none defined - create status labels with tea labels create before issue planning)"
fi

echo
echo "Issue list is capped at 100; rerun with a narrower query if the repository has more open issues."
