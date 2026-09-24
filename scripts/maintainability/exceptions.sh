#!/usr/bin/env bash

# shellcheck disable=SC2154
maintainability_validate_exceptions() {
  local rows path maximum issue reason kind actual follow_up
  rows="$(mktemp)"
  if ! python3 "$script_dir/maintainability/parse-exceptions.py" "$exceptions_file" > "$rows"; then
    rm -f "$rows"
    return 1
  fi

  local failures=0
  while IFS=$'\t' read -r path maximum issue reason; do
    kind="$(maintainability_classify_path "$path" || true)"
    if [[ -z "$kind" ]] || ! maintainability_is_regular_file_at "$repo_root" "$head_sha" "$path"; then
      echo "exception path is missing, generated, or not a regular source at head: $path" >&2
      failures=$((failures + 1))
      continue
    fi
    actual="$(maintainability_line_count_at "$repo_root" "$head_sha" "$path")"
    follow_up="$(maintainability_follow_up_threshold "$kind")"
    if (( actual <= follow_up )); then
      echo "expired maintainability exception: $path ($actual <= $follow_up)" >&2
      failures=$((failures + 1))
    elif (( actual > maximum )); then
      echo "maintainability exception maximum exceeded: $path ($actual > $maximum)" >&2
      failures=$((failures + 1))
    fi
    exception_maximum["$path"]="$maximum"
  done < "$rows"
  rm -f "$rows"
  (( failures == 0 ))
}
