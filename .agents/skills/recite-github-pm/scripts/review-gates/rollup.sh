#!/usr/bin/env bash

# Status-check reduction and evaluation are deterministic and deliberately
# independent of GitHub API orchestration. They are also used by the local
# rollup fixtures.

reduce_check_rollup() {
  jq -c '
    if type != "array" then
      error("expected a statusCheckRollup array")
    else
      map(
        . as $check
        | ($check.name // $check.context // "") as $identity
        | ($check.startedAt // $check.createdAt // $check.completedAt // "") as $timestamp
        | (($check.conclusion // $check.state // $check.status // "") | ascii_downcase) as $result
        | {
            identity: $identity,
            timestamp: $timestamp,
            completedAt: ($check.completedAt // ""),
            detailsUrl: ($check.detailsUrl // ""),
            workflowName: ($check.workflowName // ""),
            orderable: (
              $identity != ""
              and $timestamp != ""
              and (($timestamp | startswith("0001-")) | not)
            ),
            resultRank: (if ($result | IN("success", "neutral", "skipped")) then 0 else 1 end),
            check: $check
          }
      )
      | sort_by(.identity)
      | group_by(.identity)
      | map(
          if any(.[]; (.orderable | not)) then
            {
              name: (.[0].identity | if . == "" then "<unnamed>" else . end),
              status: "RECITE_AMBIGUOUS"
            }
          else
            sort_by([
              .timestamp,
              .resultRank,
              .completedAt,
              .detailsUrl,
              .workflowName,
              (.check.status // .check.state // "")
            ])
            | last.check
          end
        )
    end
  '
}

evaluate_check_rollup() {
  local checks_json="$1"
  local required="$2"
  local required_state blocking_checks

  required_state="$(printf '%s\n' "$checks_json" | jq -r --arg required "$required" '
    [.[ ]
     | select((.name // .context // "") == $required)
     | (.conclusion // .state // .status // "")
     | ascii_downcase]
    | if length == 1 then .[0] else "" end
  ')"
  if [[ "$required_state" != "success" ]]; then
    echo "required aggregate check ${required} is missing, pending, or unsuccessful" >&2
    return 1
  fi

  blocking_checks="$(printf '%s\n' "$checks_json" | jq -r '
    .[]
    | . as $check
    | (($check.conclusion // $check.state // $check.status // "") | ascii_downcase) as $result
    | select(($result | IN("success", "neutral", "skipped")) | not)
    | "\($check.name // $check.context // "<unnamed>"): \($result)"
  ')"
  if [[ -n "$blocking_checks" ]]; then
    echo "$blocking_checks" >&2
    return 1
  fi

  return 0
}
