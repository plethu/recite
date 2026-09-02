#!/usr/bin/env bash

# Pull-request title, body, branch, and integration-label validation. This
# contract is shared by the fixture entry points and the live gate.

is_valid_recite_title() {
  [[ "$1" =~ ^\[REC-[1-9][0-9]*\]\ [a-z][a-z0-9-]*(\([^[:space:]]+\))?!?:\ [^[:space:]].*$ ]]
}

issue_code_from_recite_title() {
  local title="$1"

  if is_valid_recite_title "$title" && [[ "$title" =~ ^\[REC-([1-9][0-9]*)\][[:space:]] ]]; then
    printf '%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  return 1
}

closing_issue_matches_body() {
  local body="$1"
  local issue_code="$2"

  grep -Eiq -- \
    "(^|[^[:alnum:]])(close[sd]?|fix(e[sd])?|resolve[sd]?)[[:space:]]+#${issue_code}([^[:alnum:]_]|$)" \
    <<<"$body"
}

validate_pr_metadata() {
  local pr_json="$1"
  local expected_head="$2"
  local expected_base="$3"
  local title body pr_base pr_head label_count title_issue
  local failures=0

  title="$(printf '%s\n' "$pr_json" | jq -r '.title // empty')"
  body="$(printf '%s\n' "$pr_json" | jq -r '.body // empty')"
  pr_base="$(printf '%s\n' "$pr_json" | jq -r '.baseRefName // empty')"
  pr_head="$(printf '%s\n' "$pr_json" | jq -r '.headRefName // empty')"
  label_count="$(printf '%s\n' "$pr_json" | jq '[.labels[]?.name | select(. == "workflow/integration")] | length')"

  if ! title_issue="$(issue_code_from_recite_title "$title")"; then
    echo "invalid live pull-request title: ${title:-<missing>}" >&2
    failures=$((failures + 1))
  elif [[ -z "$body" ]] || ! closing_issue_matches_body "$body" "$title_issue"; then
    echo "live pull-request body must contain Closes/Fixes/Resolves #${title_issue}" >&2
    failures=$((failures + 1))
  fi

  if [[ "$pr_base" != "$expected_base" ]]; then
    echo "live pull-request base is ${pr_base:-<missing>}, expected ${expected_base}" >&2
    failures=$((failures + 1))
  fi
  if [[ -n "$expected_head" && "$pr_head" != "$expected_head" ]]; then
    echo "live pull-request head is ${pr_head:-<missing>}, expected ${expected_head}" >&2
    failures=$((failures + 1))
  fi
  if [[ -z "$pr_head" || "$pr_head" == "main" ]]; then
    echo "live pull-request head must not be protected main or missing" >&2
    failures=$((failures + 1))
  fi

  if [[ "$pr_head" == integration/* ]]; then
    if [[ ! "$pr_head" =~ ^integration/[a-z][a-z0-9]*(\-[a-z0-9]+)*$ ]]; then
      echo "live integration pull-request head is not purpose-first: $pr_head" >&2
      failures=$((failures + 1))
    elif [[ "$label_count" != "1" ]]; then
      echo "live integration pull-request head requires workflow/integration label" >&2
      failures=$((failures + 1))
    fi
  elif [[ "$label_count" == "1" ]]; then
    echo "live workflow/integration label requires an integration head branch" >&2
    failures=$((failures + 1))
  fi
  if [[ "$label_count" == "1" && "$pr_base" != "main" ]]; then
    echo "live integration pull-request must target main" >&2
    failures=$((failures + 1))
  fi

  (( failures == 0 ))
}
