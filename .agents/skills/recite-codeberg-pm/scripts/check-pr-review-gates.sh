#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-pr-review-gates.sh <pr-number> [head-branch] [base-branch]

Read-only gate for Recite PR merges. Codeberg branch protection is the
canonical project policy. This helper verifies that policy through the API,
then applies Recite-local gates that Codeberg cannot express:
  - clean-context agent review for the current head SHA, using scripts/check-project-gates.sh
    (legacy expanded check lists are also accepted);
  - no unresolved review comments;
  - no failed or missing required commit statuses;
  - temporary single-maintainer self-review handling;
  - local checks and signed merge remain mandatory in merge-pr-signed.sh.

Maintainers are derived from Codeberg repository metadata where possible:
  - repository owner;
  - repository collaborators returned by the Codeberg API.

Environment:
  RECITE_MAINTAINERS  Comma-separated fallback/additional maintainer logins.
                      Default: plethu
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

pr_number="${1:-}"
expected_head="${2:-}"
expected_base="${3:-main}"

if [[ -z "$pr_number" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$pr_number" =~ ^[0-9]+$ ]]; then
  echo "PR number must be numeric: $pr_number" >&2
  exit 2
fi

if ! command -v tea >/dev/null 2>&1; then
  echo "tea not installed; install and authenticate tea for Codeberg before checking PR gates" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "jq not installed; install jq before checking PR gates" >&2
  exit 2
fi

failures=0

fail() {
  echo "BLOCKED: $*" >&2
  failures=$((failures + 1))
}

echo "== pull request #${pr_number} =="
pr_json="$(tea api "repos/{owner}/{repo}/pulls/${pr_number}")"

pr_state="$(printf '%s\n' "$pr_json" | jq -r '.state // empty')"
pr_base="$(printf '%s\n' "$pr_json" | jq -r '.base.ref // empty')"
pr_head="$(printf '%s\n' "$pr_json" | jq -r '.head.ref // empty')"
pr_author="$(printf '%s\n' "$pr_json" | jq -r '.user.login // empty')"
head_sha="$(printf '%s\n' "$pr_json" | jq -r '.head.sha // empty')"
mergeable="$(printf '%s\n' "$pr_json" | jq -r '.mergeable')"

printf '%s\n' "$pr_json" | jq '{
  number,
  title,
  state,
  url: (.html_url // .url),
  base: .base.ref,
  head: .head.ref,
  author: .user.login,
  headSha: .head.sha,
  mergeable
}'

[[ "$pr_state" == "open" ]] || fail "PR state is ${pr_state:-missing}, expected open"
[[ "$pr_base" == "$expected_base" ]] || fail "PR base is ${pr_base:-missing}, expected ${expected_base}"
if [[ -n "$expected_head" ]]; then
  [[ "$pr_head" == "$expected_head" ]] || fail "PR head is ${pr_head:-missing}, expected ${expected_head}"
fi
[[ "$mergeable" == "true" ]] || fail "PR is not mergeable according to Codeberg"
[[ -n "$head_sha" ]] || fail "PR head SHA is missing"

echo
echo "== base branch protection =="
branch_json="$(tea api "repos/{owner}/{repo}/branches/${pr_base}")"
printf '%s\n' "$branch_json" | jq '{
  name,
  protected,
  required_approvals,
  enable_status_check,
  status_check_contexts,
  user_can_push,
  user_can_merge,
  effective_branch_protection_name,
  require_signed_commits: (.require_signed_commits // .enable_signed_commits // null)
}'

branch_protected="$(printf '%s\n' "$branch_json" | jq -r '.protected')"
required_approvals="$(printf '%s\n' "$branch_json" | jq -r '.required_approvals // 0')"
status_checks_enabled="$(printf '%s\n' "$branch_json" | jq -r '.enable_status_check // false')"
user_can_merge="$(printf '%s\n' "$branch_json" | jq -r '.user_can_merge // false')"

[[ "$branch_protected" == "true" ]] || fail "base branch ${pr_base:-missing} is not protected"
[[ "$user_can_merge" == "true" ]] || fail "current Codeberg user cannot merge ${pr_base:-missing}"
if ! [[ "$required_approvals" =~ ^[0-9]+$ ]]; then
  fail "branch required_approvals is not numeric: ${required_approvals}"
elif (( required_approvals < 1 )); then
  fail "branch protection does not require any PR approvals"
fi

echo
echo "== maintainers =="
repo_json="$(tea api repos/{owner}/{repo})"
repo_owner="$(printf '%s\n' "$repo_json" | jq -r '.owner.login // empty')"
collaborators_json="$(tea api repos/{owner}/{repo}/collaborators)"

maintainers="$(
  {
    if [[ -n "$repo_owner" ]]; then
      printf '%s\n' "$repo_owner"
    fi
    printf '%s\n' "$collaborators_json" | jq -r '.[]?.login // empty'
    printf '%s\n' "${RECITE_MAINTAINERS:-plethu}" | tr ',' '\n'
  } | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | awk 'NF' | sort -u
)"
maintainer_count="$(printf '%s\n' "$maintainers" | awk 'NF' | wc -l | tr -d ' ')"

if [[ -z "$maintainers" ]]; then
  fail "no known maintainers were found"
else
  printf '%s\n' "$maintainers"
  echo "count: ${maintainer_count}"
fi

echo
echo "== maintainer approval =="
reviews_json="$(tea api "repos/{owner}/{repo}/pulls/${pr_number}/reviews")"
latest_approvers="$(
  printf '%s\n' "$reviews_json" | jq -r '
    [
      .[]?
      | {
          user: (.user.login // empty),
          state: (.state // "" | ascii_upcase),
          submitted: (.submitted_at // .updated_at // .created_at // "")
        }
      | select(.user != "")
    ]
    | sort_by(.user, .submitted)
    | group_by(.user)[]
    | last
    | select(.state == "APPROVED")
    | .user
  ' | sort -u
)"

approved_maintainers="$(
  comm -12 \
    <(printf '%s\n' "$maintainers" | sort -u) \
    <(printf '%s\n' "$latest_approvers" | awk 'NF' | sort -u) || true
)"

non_author_approved_maintainers="$(
  printf '%s\n' "$approved_maintainers" | awk -v author="$pr_author" 'NF && $0 != author'
)"

trusted_author="$(
  comm -12 \
    <(printf '%s\n' "$maintainers" | sort -u) \
    <(printf '%s\n' "$pr_author" | awk 'NF' | sort -u) || true
)"

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
  fail "no Codeberg approval review or trusted single-maintainer author self-review"
fi

echo
echo "== clean-context agent review =="
comments_json="$(tea api "repos/{owner}/{repo}/issues/${pr_number}/comments")"
agent_review_count="$(
  printf '%s\n' "$comments_json" | jq --arg sha "$head_sha" '
    [
      .[]?
      | select((.body // "") | contains("<!-- recite-agent-review:v1 -->"))
      | select((.body // "") | test("Agent-Review:[[:space:]]*approved"; "i"))
      | select((.body // "") | test("Context:[[:space:]]*clean"; "i"))
      | select((.body // "") | contains("Head-SHA: " + $sha))
      | select(
          ((.body // "") | contains("scripts/check-project-gates.sh"))
          or (
            ((.body // "") | contains("check-test-organization.sh"))
            and ((.body // "") | contains("cargo fmt --check"))
            and ((.body // "") | contains("cargo test"))
            and ((.body // "") | contains("cargo clippy --all-targets --all-features -- -D warnings"))
          )
        )
    ]
    | length
  '
)"

if (( agent_review_count > 0 )); then
  echo "found clean-context agent review for ${head_sha}"
else
  fail "missing clean-context agent review comment for ${head_sha} with scripts/check-project-gates.sh (or legacy expanded check list)"
fi

echo
echo "== unresolved review comments =="
review_comments_json="$(tea pulls review-comments "$pr_number" --fields id,body,reviewer,path,line,resolver,url --output json)"
unresolved_comments="$(
  printf '%s\n' "$review_comments_json" | jq -r '
    .[]?
    | select((.resolver // "") == "")
    | "#\(.id) \(.path // "(no path)"):\(.line // "-") \(.url // "")"
  '
)"

if [[ -n "$unresolved_comments" ]]; then
  printf '%s\n' "$unresolved_comments" >&2
  fail "unresolved review comments remain"
else
  echo "none"
fi

echo
echo "== reported commit statuses =="
statuses_json="$(tea api "repos/{owner}/{repo}/commits/${head_sha}/statuses")"
latest_statuses="$(
  printf '%s\n' "$statuses_json" | jq -r '
    [
      .[]?
      | {
          context: (.context // ""),
          state: (.state // "" | ascii_downcase),
          updated: (.updated_at // .created_at // "")
        }
      | select(.context != "")
    ]
    | sort_by(.context, .updated)
    | group_by(.context)[]
    | last
    | "\(.context)\t\(.state)"
  '
)"
status_count="$(printf '%s\n' "$latest_statuses" | awk 'NF' | wc -l | tr -d ' ')"
blocking_statuses="$(
  printf '%s\n' "$latest_statuses" | awk -F '\t' '$2 == "failure" || $2 == "error" || $2 == "cancelled" { print $0 }'
)"

if [[ -n "$latest_statuses" ]]; then
  printf '%s\n' "$latest_statuses"
else
  echo "none reported"
fi

if [[ -n "$blocking_statuses" ]]; then
  printf '%s\n' "$blocking_statuses" >&2
  fail "failed Codeberg commit statuses are present"
fi

required_contexts="$(printf '%s\n' "$branch_json" | jq -r '.status_check_contexts[]?')"
if [[ "$status_checks_enabled" == "true" ]]; then
  if [[ -z "$required_contexts" ]]; then
    fail "branch status checks are enabled but no required contexts are configured"
  else
    while IFS= read -r context; do
      [[ -n "$context" ]] || continue
      state="$(
        printf '%s\n' "$latest_statuses" | awk -F '\t' -v context="$context" '$1 == context { print $2; found = 1 } END { if (!found) exit 1 }' || true
      )"
      if [[ -z "$state" ]]; then
        fail "required status context ${context} is missing"
      elif [[ "$state" != "success" ]]; then
        fail "required status context ${context} is ${state}, expected success"
      fi
    done <<< "$required_contexts"
  fi
elif [[ "$status_count" == "0" ]]; then
  echo "branch status checks are disabled and no statuses are reported; local checks remain mandatory"
else
  echo "branch status checks are disabled; local checks remain mandatory"
fi

if (( failures > 0 )); then
  echo
  echo "PR #${pr_number} failed ${failures} review gate(s)." >&2
  exit 1
fi

echo
echo "PR #${pr_number} passed Recite review gates."
