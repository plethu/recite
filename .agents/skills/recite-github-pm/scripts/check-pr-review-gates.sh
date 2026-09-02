#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=.agents/skills/recite-github-pm/scripts/review-gates/rollup.sh
source "$script_dir/review-gates/rollup.sh"
# shellcheck source=.agents/skills/recite-github-pm/scripts/review-gates/metadata.sh
source "$script_dir/review-gates/metadata.sh"
# shellcheck source=.agents/skills/recite-github-pm/scripts/review-gates/live.sh
source "$script_dir/review-gates/live.sh"

usage() {
  cat <<'EOF'
Usage:
  check-pr-review-gates.sh <pr-number> [head-branch] [base-branch]
  check-pr-review-gates.sh --check-rollup <json-file> [required-check]
  check-pr-review-gates.sh --check-metadata <json-file> [head-branch] [base-branch]

Read-only gate for Recite pull-request merges. GitHub branch protection is the
canonical project policy. This helper verifies that policy through the GitHub
CLI, then applies Recite-local gates that protection cannot express:
  - human maintainer approval, with the current solo-maintainer self-review path;
  - no unresolved review threads;
  - no failed or errored reported checks.

Environment:
  RECITE_GITHUB_REPO  Repository used for GitHub operations. Default:
                      plethu/recite.
  RECITE_MAINTAINERS  Comma-separated fallback/additional maintainer logins.
                      Default: plethu
  RECITE_REQUIRED_CHECK Required aggregate check context. Default:
                        required-check.

The --check-rollup mode is a local deterministic fixture hook. It reads a
statusCheckRollup array from a JSON file, reduces duplicate check identities to
their newest result, and applies the required and blocking-state checks.
The --check-metadata mode validates the live PR metadata contract offline.
EOF
}

if [[ "${1:-}" == "--check-metadata" ]]; then
  metadata_file="${2:-}"
  metadata_head="${3:-}"
  metadata_base="${4:-main}"

  if [[ -z "$metadata_file" || ! -f "$metadata_file" ]]; then
    echo "pull-request metadata fixture is missing: ${metadata_file:-<path>}" >&2
    exit 2
  fi
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq not installed; install it before checking PR metadata fixtures" >&2
    exit 2
  fi
  if ! jq -e 'type == "object"' "$metadata_file" >/dev/null; then
    echo "pull-request metadata fixture is not a JSON object: $metadata_file" >&2
    exit 2
  fi
  metadata_json="$(<"$metadata_file")"
  if validate_pr_metadata "$metadata_json" "$metadata_head" "$metadata_base"; then
    echo "pull-request metadata passed: $metadata_file"
    exit 0
  fi
  echo "pull-request metadata blocked: $metadata_file" >&2
  exit 1
fi

if [[ "${1:-}" == "--check-rollup" ]]; then
  rollup_file="${2:-}"
  rollup_required="${3:-required-check}"

  if [[ -z "$rollup_file" || ! -f "$rollup_file" ]]; then
    echo "status-check rollup fixture is missing: ${rollup_file:-<path>}" >&2
    exit 2
  fi
  if ! command -v jq >/dev/null 2>&1; then
    echo "jq not installed; install it before checking status-rollup fixtures" >&2
    exit 2
  fi

  if ! reduced_rollup="$(reduce_check_rollup <"$rollup_file")"; then
    echo "unable to reduce status-check rollup fixture: $rollup_file" >&2
    exit 2
  fi

  if evaluate_check_rollup "$reduced_rollup" "$rollup_required"; then
    echo "status-check rollup passed: $rollup_file"
    exit 0
  fi

  echo "status-check rollup blocked: $rollup_file" >&2
  exit 1
fi

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

pr_number="${1:-}"
expected_head="${2:-}"
expected_base="${3:-main}"
repo="${RECITE_GITHUB_REPO:-plethu/recite}"
required_check="${RECITE_REQUIRED_CHECK:-required-check}"

if [[ -z "$pr_number" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$pr_number" =~ ^[0-9]+$ ]]; then
  echo "PR number must be numeric: $pr_number" >&2
  exit 2
fi

for command in gh jq; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command not installed; install it before checking PR gates" >&2
    exit 2
  fi
done

run_live_review_gate "$pr_number" "$expected_head" "$expected_base" "$repo" "$required_check"
