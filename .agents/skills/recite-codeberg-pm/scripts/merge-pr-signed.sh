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
  RECITE_SIGNED_MERGE_SKIP_CHECKS=1  Skip cargo fmt/test/clippy checks.
  RECITE_SIGNED_MERGE_SKIP_GATES=1   Skip remote review gates.
  RECITE_SIGNED_MERGE_SKIP_MARK=1    Skip Codeberg manual-merged marker.
  RECITE_SIGNED_MERGE_KEEP_HEAD=1     Keep the PR head branch after merge.

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
base_repo="$(printf '%s\n' "$pr_json" | jq -r '.base.repo.full_name // empty')"
head_repo="$(printf '%s\n' "$pr_json" | jq -r '.head.repo.full_name // empty')"

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

  echo
  echo "== cargo clippy =="
  cargo clippy --all-targets --all-features -- -D warnings
fi

echo
echo "== signed merge commit =="
git commit -S \
  -m "Merge pull request #${pr_number} from ${head_branch}" \
  -m "Reviewed locally and merged with a signed merge commit."
merge_sha="$(git rev-parse HEAD)"

echo
echo "== push ${base_branch} =="
git push origin "$base_branch"

if [[ "${RECITE_SIGNED_MERGE_SKIP_MARK:-0}" != "1" ]]; then
  echo
  echo "== verify pushed merge contains PR head =="
  git fetch origin "$base_branch"
  git merge-base --is-ancestor "$head_sha" "origin/${base_branch}"

  echo
  echo "== mark PR manually merged =="
  manual_merge_message="Manually merged by signed local merge commit ${merge_sha}."
  set +e
  manual_merge_output="$(
    tea api -X POST "repos/{owner}/{repo}/pulls/${pr_number}/merge" \
      -f Do=manually-merged \
      -f MergeCommitID="$merge_sha" \
      -f MergeMessageField="$manual_merge_message" \
      -f head_commit_id="$head_sha" 2>&1
  )"
  manual_merge_status=$?
  set -e

  if printf '%s\n' "$manual_merge_output" | grep -Fq 'not an allowed merge style'; then
    echo "manual merge is not enabled for this repository; enabling it and retrying"
    tea api -X PATCH repos/{owner}/{repo} \
      -F allow_manual_merge=true \
      -F autodetect_manual_merge=true >/dev/null

    set +e
    manual_merge_output="$(
      tea api -X POST "repos/{owner}/{repo}/pulls/${pr_number}/merge" \
        -f Do=manually-merged \
        -f MergeCommitID="$merge_sha" \
        -f MergeMessageField="$manual_merge_message" \
        -f head_commit_id="$head_sha" 2>&1
    )"
    manual_merge_status=$?
    set -e
  fi

  if (( manual_merge_status != 0 )); then
    printf '%s\n' "$manual_merge_output" >&2
    exit "$manual_merge_status"
  fi

  if [[ -n "$manual_merge_output" ]]; then
    printf '%s\n' "$manual_merge_output"
  fi

  if [[ "${RECITE_SIGNED_MERGE_KEEP_HEAD:-0}" != "1" ]]; then
    echo
    echo "== delete merged head branch =="
    if [[ -z "$base_repo" || -z "$head_repo" ]]; then
      echo "skipping branch deletion: PR response did not include base/head repository names"
    elif [[ "$head_repo" != "$base_repo" ]]; then
      echo "skipping branch deletion: PR head is from ${head_repo}, not ${base_repo}"
    elif [[ "$head_branch" == "$base_branch" || "$head_branch" == "main" ]]; then
      echo "skipping branch deletion: refusing to delete protected branch name ${head_branch}"
    else
      encoded_head_branch="$(printf '%s' "$head_branch" | jq -sRr @uri)"
      set +e
      delete_branch_output="$(
        tea api -X DELETE "repos/{owner}/{repo}/branches/${encoded_head_branch}" 2>&1
      )"
      delete_branch_status=$?
      set -e

      if (( delete_branch_status != 0 )); then
        if printf '%s\n' "$delete_branch_output" | grep -Eq '(^|[^0-9])404([^0-9]|$)|not[ -]found|does not exist'; then
          echo "head branch ${head_branch} was already deleted"
        else
          printf '%s\n' "$delete_branch_output" >&2
          exit "$delete_branch_status"
        fi
      else
        echo "deleted ${head_branch}"
      fi
    fi
  fi
fi

echo
echo "== post-merge PR state =="
post_merge_json="$(tea api "repos/{owner}/{repo}/pulls/${pr_number}")"
printf '%s\n' "$post_merge_json" | jq '{
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

if [[ "${RECITE_SIGNED_MERGE_SKIP_MARK:-0}" != "1" ]]; then
  pr_merged="$(printf '%s\n' "$post_merge_json" | jq -r '.merged')"
  pr_merge_sha="$(printf '%s\n' "$post_merge_json" | jq -r '.merge_commit_sha // empty')"
  if [[ "$pr_merged" != "true" || "$pr_merge_sha" != "$merge_sha" ]]; then
    echo "PR #${pr_number} was pushed but Codeberg did not record manual merge ${merge_sha}" >&2
    exit 1
  fi
fi
