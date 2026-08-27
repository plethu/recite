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
  local branch="${3:-integration/milestone-integration}"

  RECITE_PR_TITLE="$title" \
    RECITE_INTEGRATION_PR="$integration" \
    RECITE_BASE_REF="$base_sha" \
    RECITE_HEAD_REF=HEAD \
    RECITE_HEAD_BRANCH="$branch" \
    "$clone_root/scripts/check-git-policy.sh" "$clone_root"
}

if run_policy "[REC-163] chore: integrate milestone" 0 >/dev/null 2>&1; then
  echo "ordinary policy unexpectedly accepted a mixed-code commit range" >&2
  exit 1
fi

if ! run_policy "[REC-163] chore: integrate milestone" 1; then
  echo "integration policy rejected a valid mixed-code commit range" >&2
  exit 1
fi

if run_policy "[REC-163] chore: integrate milestone" 1 feat/milestone-integration >/dev/null 2>&1; then
  echo "integration policy accepted a non-integration head branch" >&2
  exit 1
fi

if run_policy "chore: integrate milestone" 1 >/dev/null 2>&1; then
  echo "integration policy accepted a title without the milestone issue code" >&2
  exit 1
fi

git -C "$clone_root" commit --quiet --allow-empty \
  -m "[REC-145] docs: preserve attribution rule" \
  -m "Co-Authored-By: fixture <fixture@example.invalid>"
if run_policy "[REC-163] chore: integrate milestone" 1 >/dev/null 2>&1; then
  echo "integration policy accepted an attribution trailer" >&2
  exit 1
fi

echo "Git policy integration-mode fixtures passed."
