#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  merge-pr-signed.sh <pr-number> [expected-head-branch] [expected-base-branch]

Creates a signed local merge commit for a Codeberg pull request and pushes the
base branch. This avoids Codeberg's web merge path, which cannot satisfy
Recite's signed-commit requirement.

Example:
  .agents/skills/recite-codeberg-pm/scripts/merge-pr-signed.sh 34 issue-1-workspace-split main

Environment:
  RECITE_SIGNED_MERGE_SKIP_CHECKS=1  Skip cargo fmt/test checks.
  RECITE_SIGNED_MERGE_SKIP_GATES=1   Skip remote review gates.

If checks fail after the merge is staged, inspect the tree and run:
  git merge --abort
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

pr_number="${1:-}"
expected_head_branch="${2:-}"
expected_base_branch="${3:-main}"

if [[ -z "$pr_number" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$pr_number" =~ ^[0-9]+$ ]]; then
  echo "PR number must be numeric: $pr_number" >&2
  exit 2
fi

if ! command -v tea >/dev/null 2>&1; then
  echo "tea not installed; install and authenticate tea for Codeberg before merging" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq not installed; install jq before merging" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [[ -n "$(git status --porcelain)" ]]; then
  echo "working tree is not clean; commit, stash, or discard local changes before merging" >&2
  exit 1
fi

echo "== pull request #${pr_number} =="
pr_json="$(tea api "repos/{owner}/{repo}/pulls/${pr_number}")"
printf '%s\n' "$pr_json" | jq '{
  number,
  title,
  state,
  url: (.html_url // .url),
  base: .base.ref,
  head: .head.ref,
  headSha: .head.sha,
  mergeable
}'

base_branch="$(printf '%s\n' "$pr_json" | jq -r '.base.ref // empty')"
head_branch="$(printf '%s\n' "$pr_json" | jq -r '.head.ref // empty')"
head_sha="$(printf '%s\n' "$pr_json" | jq -r '.head.sha // empty')"

if [[ -z "$base_branch" || -z "$head_branch" || -z "$head_sha" ]]; then
  echo "PR #${pr_number} is missing base, head, or head SHA in the Codeberg API response" >&2
  exit 1
fi

if [[ "$base_branch" != "$expected_base_branch" ]]; then
  echo "PR base is ${base_branch}, expected ${expected_base_branch}" >&2
  exit 1
fi

if [[ -n "$expected_head_branch" && "$head_branch" != "$expected_head_branch" ]]; then
  echo "PR head is ${head_branch}, expected ${expected_head_branch}" >&2
  exit 1
fi

if [[ "${RECITE_SIGNED_MERGE_SKIP_GATES:-0}" != "1" ]]; then
  echo
  echo "== review gates =="
  "$script_dir/check-pr-review-gates.sh" "$pr_number" "$head_branch" "$base_branch"
fi

echo
echo "== fetch =="
git fetch origin "$base_branch" "$head_branch"

commit_count="$(git rev-list --count "origin/${base_branch}..origin/${head_branch}")"
if [[ "$commit_count" == "0" ]]; then
  echo "origin/${head_branch} has no commits ahead of origin/${base_branch}" >&2
  exit 1
fi

echo
echo "== verify PR commit signatures =="
while read -r commit; do
  git verify-commit "$commit"
done < <(git rev-list --reverse "origin/${base_branch}..origin/${head_branch}")

echo
echo "== update ${base_branch} =="
git switch "$base_branch"
git merge --ff-only "origin/${base_branch}"

echo
echo "== stage merge =="
git merge --no-commit --no-ff "origin/${head_branch}"

if [[ "${RECITE_SIGNED_MERGE_SKIP_CHECKS:-0}" != "1" ]]; then
  echo
  echo "== cargo fmt --check =="
  cargo fmt --check

  echo
  echo "== cargo test =="
  cargo test
fi

echo
echo "== signed merge commit =="
git commit -S \
  -m "Merge pull request #${pr_number} from ${head_branch}" \
  -m "Reviewed locally and merged with a signed merge commit."

echo
echo "== push ${base_branch} =="
git push origin "$base_branch"

echo
echo "== post-merge PR state =="
tea api "repos/{owner}/{repo}/pulls/${pr_number}" | jq '{
  number,
  title,
  state,
  url: (.html_url // .url),
  base: .base.ref,
  head: .head.ref,
  headSha: .head.sha,
  merged,
  merge_commit_sha
}'
