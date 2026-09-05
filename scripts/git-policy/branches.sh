#!/usr/bin/env bash

# Branch naming and source/head resolution. Integration labels and PR title /
# body state are owned by metadata.sh.

allowed_branch_kinds='feat|fix|refactor|perf|ci|docs|test|build|chore|spike|release|security|integration'

git_policy_is_valid_branch_name() {
  [[ "$1" =~ ^(${allowed_branch_kinds})/[a-z][a-z0-9]*(\-[a-z0-9]+)*$ ]]
}

git_policy_is_valid_integration_branch_name() {
  [[ "$1" =~ ^integration/[a-z][a-z0-9]*(\-[a-z0-9]+)*$ ]]
}

git_policy_resolve_branch_name() {
  local repo_root="$1"
  local branch_name

  branch_name="${RECITE_BRANCH_NAME:-${RECITE_HEAD_BRANCH:-${GITHUB_HEAD_REF:-${GITHUB_REF_NAME:-}}}}"
  if [[ -z "$branch_name" ]]; then
    branch_name="$(git -C "$repo_root" branch --show-current)"
  fi
  printf '%s\n' "$branch_name"
}

git_policy_validate_branch() {
  local branch_name="$1"
  local pr_context="$2"

  if (( pr_context )) && [[ "$branch_name" == "main" ]]; then
    echo "pull-request head branch must not be protected main" >&2
    return 1
  fi

  if (( ! pr_context )) && [[ -z "$branch_name" ]]; then
    echo "detached Git policy checks require explicit branch or pull-request metadata" >&2
    return 1
  fi

  if [[ -n "$branch_name" && "$branch_name" != "main" ]]; then
    if ! git_policy_is_valid_branch_name "$branch_name"; then
      echo "invalid branch name: $branch_name" >&2
      echo "use <kind>/<short-kebab-topic> with kind in: feat, fix, refactor, perf, ci, docs, test, build, chore, spike, release, security, integration" >&2
      return 1
    fi
    echo "branch name passed: $branch_name"
  elif [[ "$branch_name" == "main" ]]; then
    echo "branch name check skipped for protected branch: main"
  else
    echo "branch name check skipped: detached HEAD without pull-request branch metadata"
  fi
}
