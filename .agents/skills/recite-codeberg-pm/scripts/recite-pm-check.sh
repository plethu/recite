#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  recite-pm-check.sh [quick]
  recite-pm-check.sh issue <number>
  recite-pm-check.sh full

Modes:
  quick   Local preflight only: git remote and tea version. This is the default.
  issue   Verify one target issue after a mutation.
  full    Broad project audit. Uses a short cache for labels and milestones.

Environment:
  RECITE_PM_CACHE_DIR          Cache directory for full-mode labels/milestones.
                               Default: /tmp/recite-pm-cache
  RECITE_PM_CACHE_TTL_SECONDS  Cache TTL in seconds. Default: 1800
  RECITE_PM_ISSUE_LIMIT        Open issue limit for full mode. Default: 100
EOF
}

if ! command -v tea >/dev/null 2>&1; then
  echo "tea not installed; install and authenticate tea for Codeberg before running Recite PM checks" >&2
  exit 2
fi

print_local_preflight() {
  echo "== git remote =="
  git remote -v

  echo
  echo "== tea version =="
  tea --version
}

cached_tea() {
  local key="$1"
  shift

  local cache_dir="${RECITE_PM_CACHE_DIR:-/tmp/recite-pm-cache}"
  local ttl="${RECITE_PM_CACHE_TTL_SECONDS:-1800}"
  local cache_file="${cache_dir}/${key}.txt"
  local now
  local mtime

  mkdir -p "$cache_dir"
  now="$(date +%s)"

  if [[ -f "$cache_file" ]]; then
    mtime="$(stat -c %Y "$cache_file" 2>/dev/null || echo 0)"
    if (( now - mtime < ttl )); then
      cat "$cache_file"
      return
    fi
  fi

  "$@" | tee "$cache_file"
}

mode="${1:-quick}"

case "$mode" in
  quick|--quick)
    print_local_preflight
    ;;
  issue|--issue)
    issue_index="${2:-}"
    if [[ -z "$issue_index" ]]; then
      echo "issue mode requires an issue number" >&2
      usage >&2
      exit 2
    fi

    print_local_preflight

    echo
    echo "== issue #${issue_index} =="
    tea issues "$issue_index" --fields index,title,state,milestone,labels,url
    ;;
  full|--full)
    print_local_preflight

    echo
    echo "== milestones =="
    cached_tea milestones tea milestones list

    echo
    echo "== labels =="
    labels_output="$(cached_tea labels tea labels list)"
    printf '%s\n' "$labels_output"

    echo
    issue_limit="${RECITE_PM_ISSUE_LIMIT:-100}"
    echo "== open issues, first ${issue_limit} =="
    tea issues list --state open --limit "$issue_limit"

    echo
    echo "== status labels defined =="
    status_labels="$(printf '%s\n' "$labels_output" | grep -E 'status/(ready|design-needed|in-progress|review|blocked)' || true)"
    if [[ -n "$status_labels" ]]; then
      printf '%s\n' "$status_labels"
    else
      echo "(none defined - create status labels with tea labels create before issue planning)"
    fi

    echo
    echo "Issue list is capped at ${issue_limit}; rerun issue mode for a specific target, or full mode with RECITE_PM_ISSUE_LIMIT adjusted."
    ;;
  -h|--help|help)
    usage
    ;;
  *)
    echo "unknown mode: $mode" >&2
    usage >&2
    exit 2
    ;;
esac
