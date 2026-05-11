#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-pr-review-gates.sh <pr-number> [head-branch] [base-branch]

Read-only gate for Recite PR merges. The gate requires:
  - an open, mergeable PR targeting the expected base/head when provided;
  - a Codeberg approval review from a known maintainer;
  - a clean-context agent review comment for the current head SHA;
  - no unresolved review comments;
  - no failed Codeberg commit statuses on the current head SHA.

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
pr_json="$(tea pulls "$pr_number" --fields index,title,state,url,base,head,mergeable --output json)"
printf '%s\n' "$pr_json" | jq '{index, title, state, url, base, head, headSha, mergeable}'

pr_state="$(printf '%s\n' "$pr_json" | jq -r '.state')"
pr_base="$(printf '%s\n' "$pr_json" | jq -r '.base')"
pr_head="$(printf '%s\n' "$pr_json" | jq -r '.head')"
head_sha="$(printf '%s\n' "$pr_json" | jq -r '.headSha')"
mergeable="$(printf '%s\n' "$pr_json" | jq -r '.mergeable')"

[[ "$pr_state" == "open" ]] || fail "PR state is ${pr_state}, expected open"
[[ "$pr_base" == "$expected_base" ]] || fail "PR base is ${pr_base}, expected ${expected_base}"
if [[ -n "$expected_head" ]]; then
  [[ "$pr_head" == "$expected_head" ]] || fail "PR head is ${pr_head}, expected ${expected_head}"
fi
[[ "$mergeable" == "true" ]] || fail "PR is not mergeable according to Codeberg"
[[ "$head_sha" != "null" && -n "$head_sha" ]] || fail "PR head SHA is missing"

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

if [[ -z "$maintainers" ]]; then
  fail "no known maintainers were found"
else
  printf '%s\n' "$maintainers"
fi

echo
echo "== maintainer approval =="
reviews_json="$(tea api "repos/{owner}/{repo}/pulls/${pr_number}/reviews")"
approvers="$(
  printf '%s\n' "$reviews_json" | jq -r '
    .[]?
    | select((.state // "" | ascii_upcase) == "APPROVED")
    | .user.login // empty
  ' | sort -u
)"

approved_maintainers="$(
  comm -12 \
    <(printf '%s\n' "$maintainers" | sort -u) \
    <(printf '%s\n' "$approvers" | awk 'NF' | sort -u) || true
)"

if [[ -n "$approved_maintainers" ]]; then
  printf '%s\n' "$approved_maintainers"
else
  fail "no Codeberg approval review from a known maintainer"
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
    ]
    | length
  '
)"

if (( agent_review_count > 0 )); then
  echo "found clean-context agent review for ${head_sha}"
else
  fail "missing clean-context agent review comment for ${head_sha}"
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
status_count="$(printf '%s\n' "$statuses_json" | jq 'length')"
blocking_statuses="$(
  printf '%s\n' "$statuses_json" | jq -r '
    .[]?
    | select((.state // "" | ascii_downcase) == "failure" or (.state // "" | ascii_downcase) == "error")
    | "\(.context // "(no context)") \(.state // "(no state)") \(.target_url // "")"
  '
)"

if [[ -n "$blocking_statuses" ]]; then
  printf '%s\n' "$blocking_statuses" >&2
  fail "failed Codeberg commit statuses are present"
elif [[ "$status_count" == "0" ]]; then
  echo "none reported; local checks remain mandatory"
else
  printf '%s\n' "$statuses_json" | jq -r '.[]? | "\(.context // "(no context)") \(.state // "(no state)")"'
fi

if (( failures > 0 )); then
  echo
  echo "PR #${pr_number} failed ${failures} review gate(s)." >&2
  exit 1
fi

echo
echo "PR #${pr_number} passed Recite review gates."
