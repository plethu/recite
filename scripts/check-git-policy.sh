#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-git-policy.sh [repo-root]

Checks the branch name and commit messages in the relevant change range.

The pull-request workflow supplies GITHUB_HEAD_REF, GITHUB_BASE_REF,
RECITE_PR_BASE_REF, RECITE_PR_TITLE, RECITE_INTEGRATION_LABEL, and
RECITE_INTEGRATION_PR. For local runs, the current branch is checked against
origin/main. Set RECITE_BASE_REF, RECITE_HEAD_REF, RECITE_HEAD_BRANCH,
RECITE_PR_TITLE, or RECITE_ISSUE_CODE to override those inputs for a focused
check. Set RECITE_INTEGRATION_PR=1 for a coordinator's milestone integration
PR when label metadata is unavailable locally; in PR context, integration mode
requires the workflow/integration label, an integration/<topic> branch, and a
main base branch.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 1 ]]; then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve git repo root from current directory" >&2
    exit 2
  fi
fi

fixture_root="$repo_root/tests/git-policy"

integration_pr="${RECITE_INTEGRATION_PR:-0}"
if [[ "$integration_pr" != "0" && "$integration_pr" != "1" ]]; then
  echo "RECITE_INTEGRATION_PR must be 0 or 1: $integration_pr" >&2
  exit 2
fi

integration_label="${RECITE_INTEGRATION_LABEL:-}"
if [[ -n "$integration_label" && "$integration_label" != "0" && "$integration_label" != "1" ]]; then
  echo "RECITE_INTEGRATION_LABEL must be 0 or 1 when set: $integration_label" >&2
  exit 2
fi

allowed_branch_kinds='feat|fix|refactor|perf|ci|docs|test|build|chore|spike|release|security|integration'

is_valid_branch_name() {
  [[ "$1" =~ ^(${allowed_branch_kinds})/[a-z][a-z0-9]*(\-[a-z0-9]+)*$ ]]
}

is_valid_integration_branch_name() {
  [[ "$1" =~ ^integration/[a-z][a-z0-9]*(\-[a-z0-9]+)*$ ]]
}

issue_code_from_pr_title() {
  local title="$1"

  if [[ "$title" =~ ^\[REC-([1-9][0-9]*)\][[:space:]] ]]; then
    printf 'REC-%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  return 1
}

is_attribution_trailer() {
  grep -Eiq \
    '^[[:space:]]*(co-?authored-by|ai-generated-by|generated-by|agent|assistant):[[:space:]]*' \
    <<<"$1"
}

sentence_count() {
  local body="$1"
  local flattened

  flattened="$(printf '%s' "$body" | tr '\n' ' ')"
  if [[ -z "${flattened//[[:space:]]/}" ]]; then
    echo 0
    return
  fi

  # A sentence terminator followed by whitespace (or the end) is a stable,
  # deliberately small check. Bodies without punctuation still count as one.
  local terminators
  terminators="$(grep -oE '[.!?]([[:space:]]|$)' <<<"$flattened" | wc -l | tr -d ' ')" || true
  if [[ "$terminators" -eq 0 ]]; then
    echo 1
  else
    echo "$terminators"
  fi
}

validate_commit_message() {
  local message="$1"
  local subject body expected_issue count

  subject="${message%%$'\n'*}"
  if [[ "$message" == *$'\n'* ]]; then
    body="${message#*$'\n'}"
  else
    body=""
  fi

  if [[ ! "$subject" =~ ^\[REC-[1-9][0-9]*\]\ [a-z][a-z0-9-]*(\([^[:space:]]+\))?!?:\ [^[:space:]].*$ ]]; then
    return 1
  fi

  expected_issue="${RECITE_ISSUE_CODE:-}"
  if [[ -n "$expected_issue" ]]; then
    expected_issue="${expected_issue#REC-}"
    if [[ "$subject" != "[REC-${expected_issue}] "* ]]; then
      return 1
    fi
  fi

  if is_attribution_trailer "$body"; then
    return 1
  fi

  count="$(sentence_count "$body")"
  [[ "$count" -le 1 ]]
}

run_fixture_checks() {
  local expected branch message_fixture message expected_result
  local failures=0

  if [[ ! -f "$fixture_root/branches.tsv" ]]; then
    echo "missing Git policy branch fixtures: $fixture_root/branches.tsv" >&2
    return 1
  fi
  if [[ ! -d "$fixture_root/commit-messages" ]]; then
    echo "missing Git policy commit fixtures: $fixture_root/commit-messages" >&2
    return 1
  fi

  while IFS=$'\t' read -r expected branch; do
    [[ -z "$expected" || "$expected" == \#* ]] && continue
    if is_valid_branch_name "$branch"; then
      actual=valid
    else
      actual=invalid
    fi
    if [[ "$actual" != "$expected" ]]; then
      echo "fixture branch expectation failed: $branch (expected $expected)" >&2
      failures=$((failures + 1))
    fi
  done < "$fixture_root/branches.tsv"

  for message_fixture in "$fixture_root"/commit-messages/*.txt; do
    [[ -e "$message_fixture" ]] || continue
    message="$(<"$message_fixture")"
    if RECITE_ISSUE_CODE='' validate_commit_message "$message"; then
      actual=valid
    else
      actual=invalid
    fi
    expected_result="${message_fixture##*/}"
    expected_result="${expected_result%%-*}"
    if [[ "$actual" != "$expected_result" ]]; then
      echo "fixture commit expectation failed: ${message_fixture##*/} (expected $expected_result)" >&2
      failures=$((failures + 1))
    fi
  done

  message="$(<"$fixture_root/commit-messages/valid-subject.txt")"
  if RECITE_ISSUE_CODE='REC-999' validate_commit_message "$message"; then
    echo "fixture commit expectation failed: mismatched issue code was accepted" >&2
    failures=$((failures + 1))
  fi

  if [[ "$(issue_code_from_pr_title '[REC-143] CI: enforce Git workflow policy')" != 'REC-143' ]]; then
    echo "fixture pull-request title expectation failed: valid title was rejected" >&2
    failures=$((failures + 1))
  fi
  if issue_code_from_pr_title 'CI: enforce Git workflow policy' >/dev/null; then
    echo "fixture pull-request title expectation failed: missing issue code was accepted" >&2
    failures=$((failures + 1))
  fi

  if (( failures > 0 )); then
    echo "Found ${failures} Git policy fixture failure(s)." >&2
    return 1
  fi

  echo "Git policy fixtures passed."
}

run_fixture_checks

branch_name="${RECITE_HEAD_BRANCH:-${GITHUB_HEAD_REF:-}}"
if [[ -z "$branch_name" ]]; then
  branch_name="$(git -C "$repo_root" branch --show-current)"
fi

if [[ -n "$branch_name" && "$branch_name" != "main" ]]; then
  if ! is_valid_branch_name "$branch_name"; then
    echo "invalid branch name: $branch_name" >&2
    echo "use <kind>/<short-kebab-topic> with kind in: feat, fix, refactor, perf, ci, docs, test, build, chore, spike, release, security, integration" >&2
    exit 1
  fi
  echo "branch name passed: $branch_name"
elif [[ "$branch_name" == "main" ]]; then
  echo "branch name check skipped for protected branch: main"
else
  echo "branch name check skipped: detached HEAD without pull-request branch metadata"
fi

pr_context=0
if [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ||
  -n "${GITHUB_HEAD_REF:-}" || -n "${RECITE_PR_TITLE:-}" ||
  -n "${RECITE_PR_BASE_REF:-}" ]]; then
  pr_context=1
fi

pr_base_ref="${RECITE_PR_BASE_REF:-${GITHUB_BASE_REF:-}}"
pr_base_context=0
if [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ||
  -n "${GITHUB_HEAD_REF:-}" || -n "${RECITE_PR_BASE_REF:-}" ||
  -n "${GITHUB_BASE_REF:-}" ]]; then
  pr_base_context=1
fi

# CI passes label presence separately so a branch/label mismatch cannot fall
# through as an ordinary PR. Local explicit integration mode remains available
# only when CI label metadata is absent.
if [[ "$integration_label" == "1" ]]; then
  if ! is_valid_integration_branch_name "$branch_name"; then
    echo "workflow/integration label requires an integration/<short-kebab-topic> head branch: ${branch_name:-<unset>}" >&2
    exit 1
  fi
  integration_pr=1
elif [[ "$integration_label" == "0" ]]; then
  if [[ "$integration_pr" == "1" ]]; then
    echo "explicit integration mode conflicts with missing workflow/integration label" >&2
    exit 1
  fi
  if (( pr_context )) && is_valid_integration_branch_name "$branch_name"; then
    echo "integration/<short-kebab-topic> pull requests require the workflow/integration label" >&2
    exit 1
  fi
elif [[ "$integration_pr" == "1" ]]; then
  if ! is_valid_integration_branch_name "$branch_name"; then
    echo "integration mode requires an integration/<short-kebab-topic> head branch: ${branch_name:-<unset>}" >&2
    exit 1
  fi
elif (( pr_context )) && is_valid_integration_branch_name "$branch_name"; then
  echo "integration/<short-kebab-topic> pull requests require the workflow/integration label" >&2
  exit 1
fi

if [[ "$integration_pr" == "1" && "$pr_base_context" == "1" ]]; then
  if [[ -z "$pr_base_ref" ]]; then
    echo "integration pull requests require an explicit main base branch" >&2
    exit 1
  fi
  if [[ "$pr_base_ref" != "main" ]]; then
    echo "integration pull requests must target main: $pr_base_ref" >&2
    exit 1
  fi
fi

if [[ -n "${RECITE_PR_TITLE:-}" ]]; then
  if ! title_issue_code="$(issue_code_from_pr_title "$RECITE_PR_TITLE")"; then
    echo "invalid pull-request title: $RECITE_PR_TITLE" >&2
    echo "expected the title to begin with [REC-N]" >&2
    exit 1
  fi

  if [[ "$integration_pr" != "1" && -n "${RECITE_ISSUE_CODE:-}" && "${RECITE_ISSUE_CODE#REC-}" != "${title_issue_code#REC-}" ]]; then
    echo "pull-request title issue code does not match RECITE_ISSUE_CODE: $title_issue_code != $RECITE_ISSUE_CODE" >&2
    exit 1
  fi

  if [[ "$integration_pr" == "1" ]]; then
    echo "integration pull-request title issue code accepted: $title_issue_code"
  else
    RECITE_ISSUE_CODE="$title_issue_code"
    export RECITE_ISSUE_CODE
  fi
fi

# A milestone integration PR is allowed to contain several valid issue codes.
# Keep the title code as the milestone tracking code, but do not use it as the
# expected code for every commit in the range.
if [[ "$integration_pr" == "1" ]]; then
  unset RECITE_ISSUE_CODE
fi

# A push to protected main is not a pull-request change range. This also lets
# a local checkout that is merely behind origin/main run the ordinary gate.
if [[ "$branch_name" == "main" && -z "${GITHUB_HEAD_REF:-}" && -z "${RECITE_BASE_REF:-}" && -z "${RECITE_HEAD_REF:-}" ]]; then
  echo "commit range check skipped for protected branch: main"
  exit 0
fi

base_ref="${RECITE_BASE_REF:-${GITHUB_BASE_REF:-origin/main}}"
head_ref="${RECITE_HEAD_REF:-HEAD}"

if [[ -n "${GITHUB_BASE_REF:-}" && "$base_ref" == "$GITHUB_BASE_REF" ]]; then
  base_ref="origin/$base_ref"
fi

if ! base_sha="$(git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" 2>/dev/null)"; then
  echo "unable to resolve Git policy base ref: $base_ref" >&2
  exit 2
fi
if ! head_sha="$(git -C "$repo_root" rev-parse --verify "${head_ref}^{commit}" 2>/dev/null)"; then
  echo "unable to resolve Git policy head ref: $head_ref" >&2
  exit 2
fi

if ! git -C "$repo_root" merge-base --is-ancestor "$base_sha" "$head_sha"; then
  echo "Git policy base is not an ancestor of head: $base_ref -> $head_ref" >&2
  exit 1
fi

mapfile -t commits < <(git -C "$repo_root" rev-list --reverse "${base_sha}..${head_sha}")
if (( ${#commits[@]} == 0 )); then
  echo "no commits in Git policy range: $base_ref..$head_ref"
  exit 0
fi

failures=0
for commit in "${commits[@]}"; do
  message="$(git -C "$repo_root" show -s --format=%B "$commit")"
  if validate_commit_message "$message"; then
    echo "commit message passed: ${commit:0:12} $(git -C "$repo_root" show -s --format=%s "$commit")"
  else
    echo "invalid commit message: $commit" >&2
    echo "expected [REC-N] <type>: <subject>, at most one body sentence, and no agent-attribution trailers" >&2
    failures=$((failures + 1))
  fi
done

if (( failures > 0 )); then
  echo "Found ${failures} Git policy commit violation(s)." >&2
  exit 1
fi

echo "Git workflow policy passed for ${#commits[@]} commit(s)."
