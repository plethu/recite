#!/usr/bin/env bash

# These values and arrays are owned by check-maintainability.sh and are shared
# deliberately so validation and changed-surface policy use one inventory.
# shellcheck disable=SC2154
maintainability_parse_baseline_rows() {
  awk -F'|' '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    /^## Inventory$/ {
      in_baseline = 1
      next
    }
    in_baseline && /^[[:space:]]*\|/ {
      path_cell = trim($2)
      if (path_cell == "Path" && trim($3) == "Lines") {
        next
      }
      if (path_cell ~ /^-+$/) {
        next
      }
      if (NF != 8) {
        print "ERROR\t" NR "\tbaseline data row must have six columns"
        next
      }
      if (path_cell !~ /^`[^`]+`$/) {
        print "ERROR\t" NR "\tbaseline path must be enclosed in backticks"
        next
      }
      sub(/^`/, "", path_cell)
      sub(/`$/, "", path_cell)
      lines = trim($3)
      kind = trim($4)
      owner = trim($5)
      disposition = trim($6)
      reason = trim($7)
      if (path_cell ~ /[\t\r]/ || lines ~ /[\t\r]/ || kind ~ /[\t\r]/ || owner ~ /[\t\r]/ || disposition ~ /[\t\r]/ || reason ~ /[\t\r]/) {
        print "ERROR\t" NR "\tbaseline data row must not contain tabs or carriage returns"
        next
      }
      print "ROW\t" NR "\t" path_cell "\t" lines "\t" kind "\t" owner "\t" disposition "\t" reason
    }
  ' "$baseline_file"
}

maintainability_valid_issue_reason() {
  local disposition="$1"
  local reason="$2"
  [[ -n "$reason" ]] || return 1

  local remaining="$reason"
  if [[ "$reason" == *'#'* ]]; then
    # Issue references are deliberately offline and syntax-only: #1, #2, ...
    # followed by punctuation, whitespace, or the end of the reason.
    remaining="$(printf '%s\n' "$reason" | sed -E 's/#[1-9][0-9]*([^[:alnum:]_]|$)/\1/g')"
    [[ "$remaining" != *'#'* ]] || return 1
    [[ "$remaining" =~ [[:alpha:]] ]] || return 1
  elif [[ "$disposition" == exception ]]; then
    return 1
  fi

  return 0
}

# shellcheck disable=SC2154
maintainability_validate_baseline() {
  local record line_number path lines kind owner disposition reason
  local expected_kind scrutiny actual_lines
  local validation_failures=0

  while IFS=$'\t' read -r record line_number path lines kind owner disposition reason; do
    [[ -n "$record" ]] || continue
    if [[ "$record" == ERROR ]]; then
      echo "invalid maintainability baseline at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
      continue
    fi
    if [[ "$record" != ROW ]]; then
      echo "invalid maintainability baseline record: $record" >&2
      validation_failures=$((validation_failures + 1))
      continue
    fi
    if [[ -n "${baseline_seen["$path"]+present}" ]]; then
      echo "duplicate maintainability baseline row at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
      continue
    fi
    baseline_seen["$path"]=1

    if [[ ! "$lines" =~ ^[1-9][0-9]*$ ]]; then
      echo "invalid line count at baseline line $line_number: $path ($lines)" >&2
      validation_failures=$((validation_failures + 1))
    fi
    if [[ -z "$owner" ]]; then
      echo "missing baseline owner at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
    fi
    case "$disposition" in
      cohesive|follow-up|review|exception) ;;
      *)
        echo "unknown baseline disposition at line $line_number: $path ($disposition)" >&2
        validation_failures=$((validation_failures + 1))
        ;;
    esac
    if ! maintainability_valid_issue_reason "$disposition" "$reason"; then
      echo "malformed baseline issue/reason at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
    fi

    expected_kind="$(maintainability_classify_path "$path" || true)"
    if [[ -z "$expected_kind" ]]; then
      echo "invalid maintainability baseline path at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
      continue
    fi
    if [[ "$kind" != "$expected_kind" ]]; then
      echo "incorrect baseline classification at line $line_number: $path (recorded $kind, expected $expected_kind)" >&2
      validation_failures=$((validation_failures + 1))
    fi
    if ! git -C "$repo_root" cat-file -e "${head_sha}:${path}" 2>/dev/null; then
      echo "baseline path is missing at head: $path" >&2
      validation_failures=$((validation_failures + 1))
      continue
    fi
    scrutiny="$(maintainability_scrutiny_threshold "$expected_kind")"
    actual_lines="$(maintainability_line_count_at "$repo_root" "$head_sha" "$path")"
    if [[ "$lines" =~ ^[1-9][0-9]*$ && "$lines" -ne "$actual_lines" ]]; then
      echo "baseline line count mismatch at line $line_number: $path (recorded $lines, actual $actual_lines)" >&2
      validation_failures=$((validation_failures + 1))
    fi
    if (( actual_lines <= scrutiny )); then
      echo "stale maintainability baseline row at line $line_number: $path is now $actual_lines lines" >&2
      validation_failures=$((validation_failures + 1))
    fi

    # shellcheck disable=SC2034
    baseline_dispositions["$path"]="$disposition"
  done < <(maintainability_parse_baseline_rows)

  return "$validation_failures"
}
