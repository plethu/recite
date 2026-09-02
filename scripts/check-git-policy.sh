#!/usr/bin/env bash
set -euo pipefail

script_dir="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/git-policy/branches.sh
source "$script_dir/git-policy/branches.sh"
# shellcheck source=scripts/git-policy/metadata.sh
source "$script_dir/git-policy/metadata.sh"
# shellcheck source=scripts/git-policy/commits.sh
source "$script_dir/git-policy/commits.sh"
# shellcheck source=scripts/git-policy/range.sh
source "$script_dir/git-policy/range.sh"

usage() {
  cat <<'EOF'
Usage:
  check-git-policy.sh [repo-root]

Checks the branch name and commit messages in the relevant change range.

The pull-request workflow supplies GITHUB_HEAD_REF, GITHUB_BASE_REF,
RECITE_PR_BASE_REF, RECITE_PR_TITLE, RECITE_PR_BODY,
RECITE_INTEGRATION_LABEL, and RECITE_INTEGRATION_PR. For local runs, the
current branch is checked against origin/main. Set RECITE_BRANCH_NAME for a
detached local checkout. Set RECITE_BASE_REF, RECITE_HEAD_REF,
RECITE_BRANCH_NAME, RECITE_HEAD_BRANCH, RECITE_PR_TITLE, or RECITE_ISSUE_CODE
to override those inputs for a focused check. Set
RECITE_INTEGRATION_PR=1 for a coordinator's local milestone integration check;
in pull-request context, every PR requires title/body metadata with a closing
issue token matching the title code. Integration mode additionally requires
label, branch, and main base metadata.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve git repo root from current directory" >&2
    exit 2
  fi
fi

fixture_root="$repo_root/tests/git-policy"

integration_pr="${RECITE_INTEGRATION_PR:-0}"
if [[ "$integration_pr" != "0" && "$integration_pr" != "1" ]]; then
  echo "RECITE_INTEGRATION_PR must be 0 or 1: $integration_pr" >&2
  exit 2
fi

integration_label="${RECITE_INTEGRATION_LABEL:-}"
if [[ -n "$integration_label" && "$integration_label" != "0" && "$integration_label" != "1" ]]; then
  echo "RECITE_INTEGRATION_LABEL must be 0 or 1 when set: $integration_label" >&2
  exit 2
fi

git_policy_run_fixture_checks "$fixture_root"

pr_context=0
if git_policy_in_pull_request_context; then
  pr_context=1
fi

branch_name="$(git_policy_resolve_branch_name "$repo_root")"
git_policy_validate_pr_context_inputs "$pr_context" "$branch_name"
git_policy_validate_branch "$branch_name" "$pr_context"

pr_base_ref="${RECITE_PR_BASE_REF:-${GITHUB_BASE_REF:-}}"
if ! integration_pr="$(git_policy_validate_integration_metadata \
  "$branch_name" "$pr_context" "$integration_pr" "$integration_label" "$pr_base_ref")"; then
  exit 1
fi

git_policy_validate_pr_metadata "$pr_context" "$integration_pr"
git_policy_run_commit_range "$repo_root" "$branch_name"
