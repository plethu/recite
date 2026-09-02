#!/usr/bin/env bash

# Live GitHub orchestration owns the order-sensitive gate and its final
# metadata/review/thread/check re-read. The sourced rollup and metadata modules
# remain pure policy helpers within this checker.

maintainer_approval_passes() {
  local reviews_json="$1"
  local review_head_sha="$2"
  local review_author="$3"
  local maintainers="$4"
  local maintainer_count="$5"
  local latest_approvers approved_maintainers non_author_approved trusted_author

  latest_approvers="$(printf '%s\n' "$reviews_json" | jq -r --arg sha "$review_head_sha" '[.[][]? | {user:(.user.login // empty),state:(.state // "" | ascii_upcase),commit:(.commit_id // empty),submitted:(.submitted_at // .updated_at // .created_at // "")}] | map(select(.user != "")) | sort_by(.user,.submitted) | group_by(.user)[] | last | select(.state == "APPROVED" and .commit == $sha) | .user' | sort -u)"
  approved_maintainers="$(comm -12 <(printf '%s\n' "$maintainers" | sort -u) <(printf '%s\n' "$latest_approvers" | awk 'NF' | sort -u) || true)"
  non_author_approved="$(printf '%s\n' "$approved_maintainers" | awk -v author="$review_author" 'NF && $0 != author')"
  trusted_author="$(comm -12 <(printf '%s\n' "$maintainers" | sort -u) <(printf '%s\n' "$review_author" | awk 'NF' | sort -u) || true)"

  if (( maintainer_count > 1 )); then
    [[ -n "$non_author_approved" ]]
  else
    [[ -n "$approved_maintainers" || -n "$trusted_author" ]]
  fi
}

run_live_review_gate() {
  local pr_number="$1"
  local expected_head="$2"
  local expected_base="$3"
  local repo="$4"
  local required_check="$5"
  local failures=0
  local pr_json pr_state pr_base pr_head pr_author head_sha mergeable merge_state_status review_decision
  local branch_json required_approvals required_contexts repo_json repo_owner repo_name
  local maintainers maintainer_count reviews_json latest_approvers approved_maintainers
  local non_author_approved_maintainers trusted_author threads_json unresolved_count reduced_rollup
  local latest_pr_json latest_head_sha latest_state latest_mergeable latest_merge_state_status
  local latest_review_decision latest_author latest_reviews_json latest_threads_json latest_unresolved_count
  local latest_reduced_rollup

  fail() {
    echo "BLOCKED: $*" >&2
    failures=$((failures + 1))
  }

  echo "== pull request #${pr_number} (${repo}) =="
  pr_json="$(gh pr view "$pr_number" --repo "$repo" --json number,title,body,labels,state,url,baseRefName,headRefName,headRefOid,author,mergeable,mergeStateStatus,reviewDecision,statusCheckRollup)"
  printf '%s\n' "$pr_json" | jq '{number,title,state,url,base:.baseRefName,head:.headRefName,author:.author.login,headSha:.headRefOid,mergeable,reviewDecision}'

  if ! validate_pr_metadata "$pr_json" "$expected_head" "$expected_base"; then
    fail "live pull-request title, head, base, label, or body metadata is invalid"
  fi

  pr_state="$(printf '%s\n' "$pr_json" | jq -r '.state // empty')"
  pr_base="$(printf '%s\n' "$pr_json" | jq -r '.baseRefName // empty')"
  pr_head="$(printf '%s\n' "$pr_json" | jq -r '.headRefName // empty')"
  pr_author="$(printf '%s\n' "$pr_json" | jq -r '.author.login // empty')"
  head_sha="$(printf '%s\n' "$pr_json" | jq -r '.headRefOid // empty')"
  mergeable="$(printf '%s\n' "$pr_json" | jq -r '.mergeable // empty')"
  merge_state_status="$(printf '%s\n' "$pr_json" | jq -r '.mergeStateStatus // empty')"
  review_decision="$(printf '%s\n' "$pr_json" | jq -r '.reviewDecision // empty' | tr '[:lower:]' '[:upper:]')"

  [[ "$pr_state" == "OPEN" ]] || fail "PR state is ${pr_state:-missing}, expected OPEN"
  [[ "$pr_base" == "$expected_base" ]] || fail "PR base is ${pr_base:-missing}, expected ${expected_base}"
  if [[ -n "$expected_head" ]]; then
    [[ "$pr_head" == "$expected_head" ]] || fail "PR head is ${pr_head:-missing}, expected ${expected_head}"
  fi
  [[ "$mergeable" == "MERGEABLE" ]] || fail "PR mergeability is ${mergeable:-missing}, expected MERGEABLE"
  if [[ -n "$merge_state_status" ]]; then
    [[ "$merge_state_status" == "CLEAN" ]] || fail "PR merge state is $merge_state_status, expected CLEAN"
  fi
  [[ -n "$head_sha" ]] || fail "PR head SHA is missing"
  [[ "$review_decision" != "CHANGES_REQUESTED" ]] || fail "GitHub reports blocking requested changes"

  echo
  echo "== base branch protection =="
  branch_json="$(gh api "repos/${repo}/branches/${pr_base}/protection" 2>/dev/null || true)"
  if [[ -z "$branch_json" ]]; then
    fail "unable to read protection for base branch ${pr_base:-missing}"
  else
    printf '%s\n' "$branch_json" | jq '{requiredApprovals:(.required_pull_request_reviews.required_approving_review_count // 0),requiredStatusChecks:(.required_status_checks.contexts // []),enforceAdmins:(.enforce_admins.enabled // false)}'
    required_approvals="$(printf '%s\n' "$branch_json" | jq -r '.required_pull_request_reviews.required_approving_review_count // 0')"
    if ! [[ "$required_approvals" =~ ^[0-9]+$ ]]; then
      fail "branch protection required_approving_review_count is not numeric"
    elif (( required_approvals == 0 )); then
      echo "solo-maintainer policy: independent approving review is not required"
    fi

    required_contexts="$(printf '%s\n' "$branch_json" | jq -r '[.required_status_checks.contexts[]?, .required_status_checks.checks[]?.context] | unique[]')"
    if ! grep -Fxq "$required_check" <<<"$required_contexts"; then
      fail "branch protection does not require aggregate check ${required_check}"
    fi
  fi

  echo
  echo "== maintainers =="
  repo_json="$(gh repo view "$repo" --json owner,name)"
  repo_owner="$(printf '%s\n' "$repo_json" | jq -r '.owner.login // empty')"
  repo_name="$(printf '%s\n' "$repo_json" | jq -r '.name // empty')"
  maintainers="$(printf '%s\n' "${RECITE_MAINTAINERS:-plethu}" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | awk 'NF' | sort -u)"
  maintainer_count="$(printf '%s\n' "$maintainers" | awk 'NF' | wc -l | tr -d ' ')"

  if [[ -z "$maintainers" ]]; then
    fail "no known maintainers were found"
  else
    printf '%s\n' "$maintainers"
    echo "count: ${maintainer_count}"
  fi

  echo
  echo "== maintainer approval =="
  reviews_json="$(gh api --paginate --slurp "repos/${repo}/pulls/${pr_number}/reviews")"
  latest_approvers="$(printf '%s\n' "$reviews_json" | jq -r --arg sha "$head_sha" '[.[][]? | {user:(.user.login // empty),state:(.state // "" | ascii_upcase),commit:(.commit_id // empty),submitted:(.submitted_at // .updated_at // .created_at // "")}] | map(select(.user != "")) | sort_by(.user,.submitted) | group_by(.user)[] | last | select(.state == "APPROVED" and .commit == $sha) | .user' | sort -u)"
  approved_maintainers="$(comm -12 <(printf '%s\n' "$maintainers" | sort -u) <(printf '%s\n' "$latest_approvers" | awk 'NF' | sort -u) || true)"
  non_author_approved_maintainers="$(printf '%s\n' "$approved_maintainers" | awk -v author="$pr_author" 'NF && $0 != author')"
  trusted_author="$(comm -12 <(printf '%s\n' "$maintainers" | sort -u) <(printf '%s\n' "$pr_author" | awk 'NF' | sort -u) || true)"

  if (( maintainer_count > 1 )); then
    if [[ -n "$non_author_approved_maintainers" ]]; then
      printf '%s\n' "$non_author_approved_maintainers"
    else
      fail "multiple maintainers are configured; approval must come from a trusted maintainer other than the PR author"
    fi
  elif [[ -n "$approved_maintainers" ]]; then
    printf '%s\n' "$approved_maintainers"
  elif [[ -n "$trusted_author" ]]; then
    printf '%s (temporary single-maintainer self-review; revisit when another maintainer is added)\n' "$trusted_author"
  else
    fail "no GitHub approval review or trusted single-maintainer author self-review"
  fi

  echo
  echo "== unresolved review threads =="
  # Keep GraphQL variables literal; `gh api` binds them with the following -F flags.
  # shellcheck disable=SC2016
  threads_json="$(gh api graphql --paginate --slurp \
    -f query='query($owner:String!,$name:String!,$number:Int!,$endCursor:String){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100,after:$endCursor){nodes{isResolved}pageInfo{hasNextPage endCursor}}}}}' \
    -F owner="$repo_owner" -F name="$repo_name" -F number="$pr_number" 2>/dev/null || true)"
  if [[ -z "$threads_json" ]] || ! printf '%s\n' "$threads_json" | jq -e 'all(.[]; ((.errors // []) | length) == 0)' >/dev/null; then
    fail "unable to read GitHub review threads"
  else
    unresolved_count="$(printf '%s\n' "$threads_json" | jq '[.[].data.repository.pullRequest.reviewThreads.nodes[]? | select(.isResolved == false)] | length')"
    if (( unresolved_count > 0 )); then
      fail "${unresolved_count} unresolved review thread(s) remain"
    else
      echo "none"
    fi
  fi

  echo
  echo "== reported checks =="
  if ! reduced_rollup="$(printf '%s\n' "$pr_json" | jq -c '.statusCheckRollup // []' | reduce_check_rollup)"; then
    fail "unable to reduce reported status checks"
  else
    if evaluate_check_rollup "$reduced_rollup" "$required_check"; then
      echo "${required_check}: success"
      echo "no failed, cancelled, or pending checks reported"
    else
      fail "newest result for a required or blocking check is not successful"
    fi
  fi

  if (( failures > 0 )); then
    echo
    echo "PR #${pr_number} failed ${failures} review gate(s)." >&2
    return 1
  fi

  echo
  echo "== final live metadata recheck =="
  if ! latest_pr_json="$(gh pr view "$pr_number" --repo "$repo" --json number,title,body,labels,state,url,baseRefName,headRefName,headRefOid,author,mergeable,mergeStateStatus,reviewDecision,statusCheckRollup 2>/dev/null)"; then
    fail "unable to refresh live pull-request metadata"
  else
    if ! validate_pr_metadata "$latest_pr_json" "$expected_head" "$expected_base"; then
      fail "live pull-request metadata changed or is invalid"
    fi
    latest_head_sha="$(printf '%s\n' "$latest_pr_json" | jq -r '.headRefOid // empty')"
    [[ "$latest_head_sha" == "$head_sha" ]] || fail "pull-request head SHA changed during gate evaluation"
    latest_state="$(printf '%s\n' "$latest_pr_json" | jq -r '.state // empty')"
    latest_mergeable="$(printf '%s\n' "$latest_pr_json" | jq -r '.mergeable // empty')"
    latest_merge_state_status="$(printf '%s\n' "$latest_pr_json" | jq -r '.mergeStateStatus // empty')"
    latest_review_decision="$(printf '%s\n' "$latest_pr_json" | jq -r '.reviewDecision // empty' | tr '[:lower:]' '[:upper:]')"
    [[ "$latest_state" == "OPEN" ]] || fail "PR state changed to ${latest_state:-missing}, expected OPEN"
    [[ "$latest_mergeable" == "MERGEABLE" ]] || fail "PR mergeability changed to ${latest_mergeable:-missing}, expected MERGEABLE"
    if [[ -n "$latest_merge_state_status" ]]; then
      [[ "$latest_merge_state_status" == "CLEAN" ]] || fail "PR merge state changed to $latest_merge_state_status, expected CLEAN"
    fi
    [[ "$latest_review_decision" != "CHANGES_REQUESTED" ]] || fail "GitHub reports newly requested changes"

    latest_author="$(printf '%s\n' "$latest_pr_json" | jq -r '.author.login // empty')"
    if ! latest_reviews_json="$(gh api --paginate --slurp "repos/${repo}/pulls/${pr_number}/reviews" 2>/dev/null)"; then
      fail "unable to refresh maintainer approvals"
    elif ! maintainer_approval_passes "$latest_reviews_json" "$latest_head_sha" "$latest_author" "$maintainers" "$maintainer_count"; then
      fail "maintainer approval is no longer valid for the current PR head"
    fi

    # shellcheck disable=SC2016
    latest_threads_json="$(gh api graphql --paginate --slurp \
      -f query='query($owner:String!,$name:String!,$number:Int!,$endCursor:String){repository(owner:$owner,name:$name){pullRequest(number:$number){reviewThreads(first:100,after:$endCursor){nodes{isResolved}pageInfo{hasNextPage endCursor}}}}}' \
      -F owner="$repo_owner" -F name="$repo_name" -F number="$pr_number" 2>/dev/null || true)"
    if [[ -z "$latest_threads_json" ]] || ! printf '%s\n' "$latest_threads_json" | jq -e 'all(.[]; ((.errors // []) | length) == 0)' >/dev/null; then
      fail "unable to refresh GitHub review threads"
    else
      latest_unresolved_count="$(printf '%s\n' "$latest_threads_json" | jq '[.[].data.repository.pullRequest.reviewThreads.nodes[]? | select(.isResolved == false)] | length')"
      (( latest_unresolved_count == 0 )) || fail "${latest_unresolved_count} unresolved review thread(s) remain"
    fi
    if ! latest_reduced_rollup="$(printf '%s\n' "$latest_pr_json" | jq -c '.statusCheckRollup // []' | reduce_check_rollup)" || ! evaluate_check_rollup "$latest_reduced_rollup" "$required_check"; then
      fail "reported checks changed or are no longer successful"
    fi
  fi

  if (( failures > 0 )); then
    echo
    echo "PR #${pr_number} failed ${failures} review gate(s)." >&2
    return 1
  fi

  echo
  echo "PR #${pr_number} passed Recite review gates."
}
