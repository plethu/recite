#!/usr/bin/env bash
set -euo pipefail

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve Git repository root from current directory" >&2
    exit 2
  fi
fi

temp_root="$(mktemp -d)"
trap 'rm -rf -- "$temp_root"' EXIT
clone_root="$temp_root/repo"

# Use a disposable detached checkout so the test exercises the same source
# shape as actions/checkout and leaves the source worktree untouched.
git clone --quiet --no-local "$repo_root" "$clone_root"
git -C "$clone_root" switch --detach --quiet HEAD
if [[ -n "$(git -C "$clone_root" branch --show-current)" ]]; then
  echo "integration fixture did not create a detached source checkout" >&2
  exit 1
fi
base_sha="$(git -C "$clone_root" rev-parse HEAD)"
# The source checkout may contain the policy script under test as an
# uncommitted change, so copy that script into the disposable clone.
cp -- "$repo_root/scripts/check-git-policy.sh" "$clone_root/scripts/check-git-policy.sh"
git -C "$clone_root" config user.name "Git policy fixture"
git -C "$clone_root" config user.email "git-policy-fixture@example.invalid"
git -C "$clone_root" config commit.gpgsign false
git -C "$clone_root" commit --quiet --allow-empty -m "[REC-143] docs: first accepted slice"
git -C "$clone_root" commit --quiet --allow-empty -m "[REC-144] docs: second accepted slice"

run_policy() {
  local title="$1"
  local integration="$2"
  local label="$3"
  local branch="${4:-integration/milestone-integration}"
  local base_branch="${5-main}"
  local body="${6-Closes #163}"

  # The clone is detached like actions/checkout; the explicit environment
  # values carry the pull-request metadata that Actions would provide.
  RECITE_PR_TITLE="$title" \
    RECITE_PR_BODY="$body" \
    RECITE_INTEGRATION_PR="$integration" \
    RECITE_INTEGRATION_LABEL="$label" \
    RECITE_PR_BASE_REF="$base_branch" \
    RECITE_BASE_REF="$base_sha" \
    RECITE_HEAD_REF=HEAD \
    GITHUB_EVENT_NAME=pull_request \
    GITHUB_HEAD_REF="$branch" \
    GITHUB_BASE_REF="$base_branch" \
    "$clone_root/scripts/check-git-policy.sh" "$clone_root"
}

run_policy_without_label() {
  local title="$1"
  local branch="${2:-integration/milestone-integration}"

  env -u RECITE_INTEGRATION_LABEL \
    RECITE_PR_TITLE="$title" \
    RECITE_INTEGRATION_PR=0 \
    RECITE_BASE_REF="$base_sha" \
    RECITE_HEAD_REF=HEAD \
    GITHUB_EVENT_NAME=pull_request \
    GITHUB_HEAD_REF="$branch" \
    GITHUB_BASE_REF=main \
    "$clone_root/scripts/check-git-policy.sh" "$clone_root"
}

if run_policy "[REC-163] chore: integrate milestone" 0 0 feat/milestone-integration >/dev/null 2>&1; then
  echo "ordinary policy unexpectedly accepted a mixed-code commit range" >&2
  exit 1
fi

if ! run_policy "[REC-163] chore: integrate milestone" 0 1; then
  echo "matching integration label and branch rejected a valid mixed-code commit range" >&2
  exit 1
fi

if ! run_policy "[REC-163] docs: close integration contract gaps" 0 1; then
  echo "current integration PR title/body was rejected" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration release >/dev/null 2>&1; then
  echo "integration PR targeting a non-main base was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration '' >/dev/null 2>&1; then
  echo "integration PR without base metadata was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration main "Closes #164" >/dev/null 2>&1; then
  echo "integration PR with a mismatched closing issue was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration main "Closes #163." >/dev/null 2>&1; then
  :
else
  echo "integration PR with valid punctuation after its closing issue was rejected" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration main "Closes #163abc" >/dev/null 2>&1; then
  echo "integration PR with an alphanumeric closing-issue suffix was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 integration/milestone-integration main "Closes #163_issue" >/dev/null 2>&1; then
  echo "integration PR with an underscored closing-issue suffix was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 0 >/dev/null 2>&1; then
  echo "integration branch without its label was accepted in PR context" >&2
  exit 1
fi

if run_policy_without_label "[REC-163] chore: integrate milestone" >/dev/null 2>&1; then
  echo "integration branch without label metadata was accepted in PR context" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 1 feat/milestone-integration >/dev/null 2>&1; then
  echo "integration label on a non-integration head branch was accepted" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 1 0 >/dev/null 2>&1; then
  echo "explicit integration mode overrode an explicit missing label" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 1 '' >/dev/null 2>&1; then
  echo "integration mode without label metadata was accepted in PR context" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 0 0 main >/dev/null 2>&1; then
  echo "pull request with protected main as its head branch was accepted" >&2
  exit 1
fi

if run_policy "" 0 0 feat/milestone-integration >/dev/null 2>&1; then
  echo "pull request without a title was accepted" >&2
  exit 1
fi

if env -u GITHUB_HEAD_REF -u GITHUB_REF_NAME \
  RECITE_PR_TITLE="[REC-163] chore: integrate milestone" \
  RECITE_PR_BODY="Closes #163" \
  RECITE_INTEGRATION_PR=0 \
  RECITE_INTEGRATION_LABEL=0 \
  RECITE_PR_BASE_REF=main \
  RECITE_BASE_REF="$base_sha" \
  RECITE_HEAD_REF=HEAD \
  GITHUB_EVENT_NAME=pull_request \
  GITHUB_BASE_REF=main \
  "$clone_root/scripts/check-git-policy.sh" "$clone_root" >/dev/null 2>&1; then
  echo "pull request without source/head metadata was accepted" >&2
  exit 1
fi

if run_policy "chore: integrate milestone" 0 1 >/dev/null 2>&1; then
  echo "integration policy accepted a title without the milestone issue code" >&2
  exit 1
fi

if ! env -u RECITE_INTEGRATION_LABEL -u RECITE_PR_BASE_REF -u GITHUB_EVENT_NAME -u GITHUB_HEAD_REF -u GITHUB_REF_NAME -u GITHUB_BASE_REF \
  RECITE_PR_TITLE="[REC-163] chore: integrate milestone" \
  RECITE_INTEGRATION_PR=1 \
  RECITE_BASE_REF="$base_sha" \
  RECITE_HEAD_REF=HEAD \
  RECITE_HEAD_BRANCH=integration/milestone-integration \
  "$clone_root/scripts/check-git-policy.sh" "$clone_root" >/dev/null; then
  echo "local explicit integration mode without label metadata was rejected" >&2
  exit 1
fi

if env -u RECITE_INTEGRATION_LABEL -u RECITE_PR_TITLE -u RECITE_PR_BODY \
  -u RECITE_BRANCH_NAME -u RECITE_HEAD_BRANCH -u GITHUB_EVENT_NAME \
  -u GITHUB_HEAD_REF -u GITHUB_REF_NAME -u GITHUB_BASE_REF -u RECITE_PR_BASE_REF \
  RECITE_INTEGRATION_PR=0 \
  RECITE_BASE_REF="$base_sha" \
  RECITE_HEAD_REF=HEAD \
  "$clone_root/scripts/check-git-policy.sh" "$clone_root" >/dev/null 2>&1; then
  echo "detached commit-range check without branch metadata was accepted" >&2
  exit 1
fi

if ! env -u RECITE_INTEGRATION_LABEL -u RECITE_PR_TITLE -u RECITE_PR_BODY \
  -u RECITE_HEAD_BRANCH -u GITHUB_EVENT_NAME -u GITHUB_HEAD_REF \
  -u GITHUB_REF_NAME -u GITHUB_BASE_REF -u RECITE_PR_BASE_REF \
  RECITE_INTEGRATION_PR=0 \
  RECITE_BRANCH_NAME=chore/local-detached-check \
  RECITE_BASE_REF="$base_sha" \
  RECITE_HEAD_REF=HEAD \
  "$clone_root/scripts/check-git-policy.sh" "$clone_root" >/dev/null; then
  echo "detached local check with RECITE_BRANCH_NAME was rejected" >&2
  exit 1
fi

git -C "$clone_root" commit --quiet --allow-empty \
  -m "[REC-145] docs: preserve attribution rule" \
  -m "Co-Authored-By: fixture <fixture@example.invalid>"
if run_policy "[REC-163] chore: integrate milestone" 0 1 >/dev/null 2>&1; then
  echo "integration policy accepted an attribution trailer" >&2
  exit 1
fi

echo "Git policy integration-mode fixtures passed."
