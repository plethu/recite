#!/usr/bin/env bash

# Commit-message validation and its deterministic fixtures. Pull-request title
# and integration metadata live in metadata.sh so this module has one owner.

git_policy_is_valid_commit_subject() {
  [[ "$1" =~ ^\[REC-[1-9][0-9]*\]\ [a-z][a-z0-9-]*(\([^[:space:]]+\))?!?:\ [^[:space:]].*$ ]]
}

git_policy_is_attribution_trailer() {
  grep -Eiq \
    '^[[:space:]]*(co-?authored-by|ai-generated-by|generated-by|agent|assistant):[[:space:]]*' \
    <<<"$1"
}

git_policy_sentence_count() {
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

git_policy_validate_commit_message() {
  local message="$1"
  local subject body expected_issue count

  subject="${message%%$'\n'*}"
  if [[ "$message" == *$'\n'* ]]; then
    body="${message#*$'\n'}"
  else
    body=""
  fi

  if ! git_policy_is_valid_commit_subject "$subject"; then
    return 1
  fi

  expected_issue="${RECITE_ISSUE_CODE:-}"
  if [[ -n "$expected_issue" ]]; then
    expected_issue="${expected_issue#REC-}"
    if [[ "$subject" != "[REC-${expected_issue}] "* ]]; then
      return 1
    fi
  fi

  if git_policy_is_attribution_trailer "$body"; then
    return 1
  fi

  count="$(git_policy_sentence_count "$body")"
  [[ "$count" -le 1 ]]
}

git_policy_run_fixture_checks() {
  local fixture_root="$1"
  local expected branch message_fixture message expected_result actual
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
    if git_policy_is_valid_branch_name "$branch"; then
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
    if RECITE_ISSUE_CODE='' git_policy_validate_commit_message "$message"; then
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
  if RECITE_ISSUE_CODE='REC-999' git_policy_validate_commit_message "$message"; then
    echo "fixture commit expectation failed: mismatched issue code was accepted" >&2
    failures=$((failures + 1))
  fi

  if [[ "$(git_policy_issue_code_from_pr_title '[REC-143] ci: enforce Git workflow policy')" != 'REC-143' ]]; then
    echo "fixture pull-request title expectation failed: valid unscoped title was rejected" >&2
    failures=$((failures + 1))
  fi
  if [[ "$(git_policy_issue_code_from_pr_title '[REC-143] ci(policy): enforce Git workflow policy')" != 'REC-143' ]]; then
    echo "fixture pull-request title expectation failed: valid scoped title was rejected" >&2
    failures=$((failures + 1))
  fi
  if [[ "$(git_policy_issue_code_from_pr_title '[REC-163] docs: close integration contract gaps')" != 'REC-163' ]]; then
    echo "fixture pull-request title expectation failed: current integration title was rejected" >&2
    failures=$((failures + 1))
  fi
  if git_policy_issue_code_from_pr_title '[REC-163] integrate milestone' >/dev/null; then
    echo "fixture pull-request title expectation failed: missing conventional separator was accepted" >&2
    failures=$((failures + 1))
  fi
  if git_policy_issue_code_from_pr_title 'CI: enforce Git workflow policy' >/dev/null; then
    echo "fixture pull-request title expectation failed: missing issue code was accepted" >&2
    failures=$((failures + 1))
  fi

  if (( failures > 0 )); then
    echo "Found ${failures} Git policy fixture failure(s)." >&2
    return 1
  fi

  echo "Git policy fixtures passed."
}
