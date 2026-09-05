#!/usr/bin/env bash

# Git ref resolution, ancestry, and commit-range validation. The caller has
# already validated branch and PR/integration metadata before entering here.

git_policy_run_commit_range() {
  local repo_root="$1"
  local branch_name="$2"
  local base_ref head_ref base_sha head_sha message commit
  local commits=()
  local failures=0

  # A push to protected main is not a pull-request change range. This also lets
  # a local checkout that is merely behind origin/main run the ordinary gate.
  if [[ "$branch_name" == "main" && -z "${GITHUB_HEAD_REF:-}" && -z "${RECITE_BASE_REF:-}" && -z "${RECITE_HEAD_REF:-}" ]]; then
    echo "commit range check skipped for protected branch: main"
    return 0
  fi

  base_ref="${RECITE_BASE_REF:-${GITHUB_BASE_REF:-origin/main}}"
  head_ref="${RECITE_HEAD_REF:-HEAD}"

  if [[ -n "${GITHUB_BASE_REF:-}" && "$base_ref" == "$GITHUB_BASE_REF" ]]; then
    base_ref="origin/$base_ref"
  fi

  if ! base_sha="$(git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" 2>/dev/null)"; then
    echo "unable to resolve Git policy base ref: $base_ref" >&2
    return 2
  fi
  if ! head_sha="$(git -C "$repo_root" rev-parse --verify "${head_ref}^{commit}" 2>/dev/null)"; then
    echo "unable to resolve Git policy head ref: $head_ref" >&2
    return 2
  fi

  if ! git -C "$repo_root" merge-base --is-ancestor "$base_sha" "$head_sha"; then
    echo "Git policy base is not an ancestor of head: $base_ref -> $head_ref" >&2
    return 1
  fi

  mapfile -t commits < <(git -C "$repo_root" rev-list --reverse "${base_sha}..${head_sha}")
  if (( ${#commits[@]} == 0 )); then
    echo "no commits in Git policy range: $base_ref..$head_ref"
    return 0
  fi

  for commit in "${commits[@]}"; do
    message="$(git -C "$repo_root" show -s --format=%B "$commit")"
    if git_policy_validate_commit_message "$message"; then
      echo "commit message passed: ${commit:0:12} $(git -C "$repo_root" show -s --format=%s "$commit")"
    else
      echo "invalid commit message: $commit" >&2
      echo "expected [REC-N] <type>: <subject>, at most one body sentence, and no agent-attribution trailers" >&2
      failures=$((failures + 1))
    fi
  done

  if (( failures > 0 )); then
    echo "Found ${failures} Git policy commit violation(s)." >&2
    return 1
  fi

  echo "Git workflow policy passed for ${#commits[@]} commit(s)."
}
