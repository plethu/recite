#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  recite-pm-check.sh [quick]
  recite-pm-check.sh issue <number>
  recite-pm-check.sh full

Modes:
  quick   Local preflight only: git remote and GitHub CLI version. This is the default.
  issue   Verify one target issue after a mutation.
  full    Broad project audit. Uses a short cache for labels and milestones.

Environment:
  RECITE_GITHUB_REPO          Repository used for GitHub operations. Default: plethu/recite
  RECITE_PM_CACHE_DIR         Cache directory for full-mode labels/milestones.
                              Default: /tmp/recite-pm-cache
  RECITE_PM_CACHE_TTL_SECONDS Cache TTL in seconds. Default: 1800
  RECITE_PM_ISSUE_LIMIT       Open issue limit for full mode. Default: 100
EOF
}

repo="${RECITE_GITHUB_REPO:-plethu/recite}"

if ! command -v gh >/dev/null 2>&1; then
  echo "gh not installed; install and authenticate the GitHub CLI before running Recite PM checks" >&2
  exit 2
fi

print_local_preflight() {
  echo "== git remote =="
  git remote -v

  echo
  echo "== GitHub CLI version =="
  gh --version | head -n 1

  echo
  echo "== GitHub authentication =="
  gh auth status --hostname github.com
}

cached_api() {
  local key="$1"
  shift

  local cache_dir="${RECITE_PM_CACHE_DIR:-/tmp/recite-pm-cache}"
  local ttl="${RECITE_PM_CACHE_TTL_SECONDS:-1800}"
  local cache_file="${cache_dir}/${key}.json"
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
    issue_number="${2:-}"
    if [[ -z "$issue_number" ]]; then
      echo "issue mode requires an issue number" >&2
      usage >&2
      exit 2
    fi

    print_local_preflight

    echo
    echo "== issue #${issue_number} =="
    gh issue view "$issue_number" --repo "$repo" --json number,title,state,milestone,labels,url
    ;;
  full|--full)
    print_local_preflight

    echo
    echo "== milestones =="
    milestones_json="$(cached_api milestones gh api --paginate --slurp "repos/${repo}/milestones?state=all&per_page=100")"
    printf '%s\n' "$milestones_json" | jq -r '.[][]? | [.number,.title,.state] | @tsv'

    echo
    echo "== labels =="
    labels_json="$(cached_api labels gh api --paginate --slurp "repos/${repo}/labels?per_page=100")"
    printf '%s\n' "$labels_json" | jq -r '.[][]? | [.name,.color] | @tsv'

    echo
    issue_limit="${RECITE_PM_ISSUE_LIMIT:-100}"
    echo "== open issues, first ${issue_limit} =="
    gh issue list --repo "$repo" --state open --limit "$issue_limit" --json number,title,state,milestone,labels,url

    echo
    echo "== status labels defined =="
    status_labels="$(printf '%s\n' "$labels_json" | jq -r '.[][]?.name // empty' | grep -E '^status/(ready|design-needed|in-progress|review|blocked)$' || true)"
    if [[ -n "$status_labels" ]]; then
      printf '%s\n' "$status_labels"
    else
      echo "(none defined - create status labels with gh label create before issue planning)"
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
