#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-pr-review-gates.sh <pr-number> [head-branch] [base-branch]

Read-only gate for Recite pull-request merges. GitHub branch protection is the
canonical project policy. This helper verifies that policy through the GitHub
CLI, then applies Recite-local gates that protection cannot express:
  - clean-context agent review for the current head SHA;
  - maintainer approval, with the current solo-maintainer self-review path;
  - no unresolved review threads;
  - no failed or errored reported checks.

Environment:
  RECITE_GITHUB_REPO  Repository used for GitHub operations. Default:
                      plethu/recite.
  RECITE_MAINTAINERS  Comma-separated fallback/additional maintainer logins.
                      Default: plethu
  RECITE_REVIEWERS    Comma-separated GitHub logins allowed to record the
                      clean-context agent review. Default: RECITE_MAINTAINERS.
  RECITE_REQUIRED_CHECK Required aggregate check context. Default:
                        required-check.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

pr_number="${1:-}"
expected_head="${2:-}"
expected_base="${3:-main}"
repo="${RECITE_GITHUB_REPO:-plethu/recite}"
required_check="${RECITE_REQUIRED_CHECK:-required-check}"

if [[ -z "$pr_number" ]]; then
  usage >&2
  exit 2
fi

if [[ ! "$pr_number" =~ ^[0-9]+$ ]]; then
  echo "PR number must be numeric: $pr_number" >&2
  exit 2
fi

for command in gh jq; do
  if ! command -v "$command" >/dev/null 2>&1; then
    echo "$command not installed; install it before checking PR gates" >&2
    exit 2
  fi
done

failures=0
fail() {
  echo "BLOCKED: $*" >&2
  failures=$((failures + 1))
}

echo "== pull request #${pr_number} (${repo}) =="
pr_json="$(gh pr view "$pr_number" --repo "$repo" --json number,title,state,url,baseRefName,headRefName,headRefOid,author,mergeable,reviewDecision,statusCheckRollup)"
printf '%s\n' "$pr_json" | jq '{number,title,state,url,base:.baseRefName,head:.headRefName,author:.author.login,headSha:.headRefOid,mergeable,reviewDecision}'

pr_state="$(printf '%s\n' "$pr_json" | jq -r '.state // empty')"
pr_base="$(printf '%s\n' "$pr_json" | jq -r '.baseRefName // empty')"
pr_head="$(printf '%s\n' "$pr_json" | jq -r '.headRefName // empty')"
pr_author="$(printf '%s\n' "$pr_json" | jq -r '.author.login // empty')"
head_sha="$(printf '%s\n' "$pr_json" | jq -r '.headRefOid // empty')"
mergeable="$(printf '%s\n' "$pr_json" | jq -r '.mergeable // empty')"

[[ "$pr_state" == "OPEN" ]] || fail "PR state is ${pr_state:-missing}, expected OPEN"
[[ "$pr_base" == "$expected_base" ]] || fail "PR base is ${pr_base:-missing}, expected ${expected_base}"
if [[ -n "$expected_head" ]]; then
  [[ "$pr_head" == "$expected_head" ]] || fail "PR head is ${pr_head:-missing}, expected ${expected_head}"
fi
[[ "$mergeable" == "MERGEABLE" ]] || fail "PR mergeability is ${mergeable:-missing}, expected MERGEABLE"
[[ -n "$head_sha" ]] || fail "PR head SHA is missing"

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
reviewers="$(printf '%s\n' "${RECITE_REVIEWERS:-${RECITE_MAINTAINERS:-plethu}}" | tr ',' '\n' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' | awk 'NF' | sort -u)"
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
latest_approvers="$(printf '%s\n' "$reviews_json" | jq -r '[.[][]? | {user:(.user.login // empty),state:(.state // "" | ascii_upcase),submitted:(.submitted_at // .updated_at // .created_at // "")}] | map(select(.user != "")) | sort_by(.user,.submitted) | group_by(.user)[] | last | select(.state == "APPROVED") | .user' | sort -u)"
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
echo "== clean-context agent review =="
comments_json="$(gh api --paginate --slurp "repos/${repo}/issues/${pr_number}/comments")"
reviewers_json="$(printf '%s\n' "$reviewers" | jq -Rsc 'split("\n") | map(select(length > 0))')"
agent_review_count="$(printf '%s\n' "$comments_json" | jq --arg sha "$head_sha" --argjson reviewers "$reviewers_json" '[.[][]? | select(.user.login as $login | $reviewers | index($login)) | select((.body // "") | contains("<!-- recite-agent-review:v1 -->")) | select((.body // "") | test("Agent-Review:[[:space:]]*approved"; "i")) | select((.body // "") | test("Context:[[:space:]]*clean"; "i")) | select((.body // "") | contains("Head-SHA: " + $sha)) | select((.body // "") | contains("mise run verify"))] | length')"
if (( agent_review_count > 0 )); then
  echo "found clean-context agent review for ${head_sha}"
else
  fail "missing clean-context agent review comment for ${head_sha} with mise run verify"
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
required_check_success="$(printf '%s\n' "$pr_json" | jq --arg required "$required_check" '[.statusCheckRollup[]? | select((.name // .context // "") == $required) | select((.conclusion // .state // "") | ascii_downcase == "success")] | length')"
if (( required_check_success == 0 )); then
  fail "required aggregate check ${required_check} is missing, pending, or unsuccessful"
else
  echo "${required_check}: success"
fi

blocking_checks="$(printf '%s\n' "$pr_json" | jq '[.statusCheckRollup[]? | select((.conclusion // .state // "") | ascii_downcase | IN("failure","error","cancelled","timed_out","action_required","stale"))] | length')"
if (( blocking_checks > 0 )); then
  fail "${blocking_checks} failed or cancelled GitHub check(s) are present"
else
  echo "no failed or cancelled checks reported"
fi

if (( failures > 0 )); then
  echo
  echo "PR #${pr_number} failed ${failures} review gate(s)." >&2
  exit 1
fi

echo
echo "PR #${pr_number} passed Recite review gates."
