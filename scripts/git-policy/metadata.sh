#!/usr/bin/env bash

# Pull-request and integration metadata state. This is deliberately separate
# from branch spelling and commit-message validation so the integration mode
# cannot be inferred from statement order in the entrypoint.

git_policy_is_valid_pr_title() {
  [[ "$1" =~ ^\[REC-[1-9][0-9]*\]\ [a-z][a-z0-9-]*(\([^[:space:]]+\))?!?:\ [^[:space:]].*$ ]]
}

git_policy_issue_code_from_pr_title() {
  local title="$1"

  if git_policy_is_valid_pr_title "$title" && [[ "$title" =~ ^\[REC-([1-9][0-9]*)\][[:space:]] ]]; then
    printf 'REC-%s\n' "${BASH_REMATCH[1]}"
    return 0
  fi

  return 1
}

git_policy_closing_issue_matches_body() {
  local body="$1"
  local issue_code="$2"

  grep -Eiq -- \
    "(^|[^[:alnum:]])(close[sd]?|fix(e[sd])?|resolve[sd]?)[[:space:]]+#${issue_code}([^[:alnum:]_]|$)" \
    <<<"$body"
}

git_policy_in_pull_request_context() {
  [[ "${GITHUB_EVENT_NAME:-}" == "pull_request" ||
    -n "${GITHUB_HEAD_REF:-}" || -n "${RECITE_PR_BASE_REF:-}" ||
    -n "${GITHUB_BASE_REF:-}" ]]
}

git_policy_validate_pr_context_inputs() {
  local pr_context="$1"
  local branch_name="$2"

  if (( pr_context )) && [[ -z "${RECITE_PR_TITLE:-}" ]]; then
    echo "pull-request context requires RECITE_PR_TITLE" >&2
    return 1
  fi

  if (( pr_context )) && [[ -z "${RECITE_BRANCH_NAME:-}" &&
    -z "${RECITE_HEAD_BRANCH:-}" && -z "${GITHUB_HEAD_REF:-}" ]]; then
    echo "pull-request context requires source/head branch metadata" >&2
    return 1
  fi

  if (( pr_context )) && [[ "$branch_name" == "main" ]]; then
    echo "pull-request head branch must not be protected main" >&2
    return 1
  fi
}

git_policy_validate_integration_metadata() {
  local branch_name="$1"
  local pr_context="$2"
  local integration_pr="$3"
  local integration_label="$4"
  local pr_base_ref="$5"

  # CI passes label presence separately so a branch/label mismatch cannot fall
  # through as an ordinary PR. Local explicit integration mode remains available
  # only when CI label metadata is absent.
  if (( pr_context )) && [[ "$integration_pr" == "1" && -z "$integration_label" ]]; then
    echo "pull-request integration mode requires workflow/integration label metadata" >&2
    return 1
  fi

  if [[ "$integration_label" == "1" ]]; then
    if ! git_policy_is_valid_integration_branch_name "$branch_name"; then
      echo "workflow/integration label requires an integration/<short-kebab-topic> head branch: ${branch_name:-<unset>}" >&2
      return 1
    fi
    integration_pr=1
  elif [[ "$integration_label" == "0" ]]; then
    if [[ "$integration_pr" == "1" ]]; then
      echo "explicit integration mode conflicts with missing workflow/integration label" >&2
      return 1
    fi
    if (( pr_context )) && git_policy_is_valid_integration_branch_name "$branch_name"; then
      echo "integration/<short-kebab-topic> pull requests require the workflow/integration label" >&2
      return 1
    fi
  elif [[ "$integration_pr" == "1" ]]; then
    if ! git_policy_is_valid_integration_branch_name "$branch_name"; then
      echo "integration mode requires an integration/<short-kebab-topic> head branch: ${branch_name:-<unset>}" >&2
      return 1
    fi
  elif (( pr_context )) && git_policy_is_valid_integration_branch_name "$branch_name"; then
    echo "integration/<short-kebab-topic> pull requests require the workflow/integration label" >&2
    return 1
  fi

  if [[ "$integration_pr" == "1" && "$pr_context" == "1" ]]; then
    if [[ -z "$pr_base_ref" ]]; then
      echo "integration pull requests require an explicit main base branch" >&2
      return 1
    fi
    if [[ "$pr_base_ref" != "main" ]]; then
      echo "integration pull requests must target main: $pr_base_ref" >&2
      return 1
    fi
  fi

  printf '%s\n' "$integration_pr"
}

git_policy_validate_pr_metadata() {
  local pr_context="$1"
  local integration_pr="$2"
  local title_issue_code

  if [[ -n "${RECITE_PR_TITLE:-}" ]]; then
    if ! title_issue_code="$(git_policy_issue_code_from_pr_title "$RECITE_PR_TITLE")"; then
      echo "invalid pull-request title: $RECITE_PR_TITLE" >&2
      echo "expected [REC-N] <type>(optional-scope): <concise subject>" >&2
      return 1
    fi

    if (( pr_context )); then
      if [[ -z "${RECITE_PR_BODY:-}" ]]; then
        echo "pull-request context requires RECITE_PR_BODY with a closing issue" >&2
        return 1
      fi
      if ! git_policy_closing_issue_matches_body "$RECITE_PR_BODY" "${title_issue_code#REC-}"; then
        echo "pull-request body must contain Closes/Fixes/Resolves #${title_issue_code#REC-}" >&2
        return 1
      fi
    fi

    if [[ "$integration_pr" != "1" && -n "${RECITE_ISSUE_CODE:-}" && "${RECITE_ISSUE_CODE#REC-}" != "${title_issue_code#REC-}" ]]; then
      echo "pull-request title issue code does not match RECITE_ISSUE_CODE: $title_issue_code != $RECITE_ISSUE_CODE" >&2
      return 1
    fi

    if [[ "$integration_pr" == "1" ]]; then
      echo "integration pull-request title issue code accepted: $title_issue_code"
    else
      RECITE_ISSUE_CODE="$title_issue_code"
      export RECITE_ISSUE_CODE
    fi
  fi

  # A milestone integration PR is allowed to contain several valid issue
  # codes. Keep the title code as the milestone tracking code, but do not use
  # it as the expected code for every commit in the range.
  if [[ "$integration_pr" == "1" ]]; then
    unset RECITE_ISSUE_CODE
  fi
}
