#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-maintainability.sh [base-ref [head-ref]] [--full]

Checks changed handwritten Rust for maintainability regressions. Line counts
are review triggers, not automatic split rules:
  production Rust: scrutiny >250, follow-up >400
  test/support Rust: scrutiny >350, follow-up >500

Unchanged oversized files are reported as legacy debt and pass. New or newly
triggered files must be recorded in docs/maintainability-baseline.md. A file
that crosses or grows above its follow-up threshold fails unless its baseline
row explicitly uses the local `exception` disposition with an issue and
reason. Baseline rows are validated against the checked-out head; use --full
for a repository-wide inventory and complete baseline validation.
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi

repo_root="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  echo "unable to resolve Git repository root" >&2
  exit 2
}

full_scan=0
refs=()
for arg in "$@"; do
  if [[ "$arg" == "--full" ]]; then
    full_scan=1
  else
    refs+=("$arg")
  fi
done
if (( ${#refs[@]} > 2 )); then
  usage >&2
  exit 2
fi

base_ref="${refs[0]:-${RECITE_BASE_REF:-origin/main}}"
head_ref="${refs[1]:-${RECITE_HEAD_REF:-HEAD}}"
if ! head_sha="$(git -C "$repo_root" rev-parse --verify "${head_ref}^{commit}" 2>/dev/null)"; then
  echo "unable to resolve maintainability head ref: $head_ref" >&2
  exit 2
fi

base_sha=""
empty_base=0
if (( ! full_scan )); then
  if [[ "$base_ref" =~ ^0{40}$ ]]; then
    base_sha="$(git -C "$repo_root" hash-object -t tree /dev/null)"
    empty_base=1
    echo "zero/initial base SHA; using the empty tree $base_sha"
  elif ! base_sha="$(git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" 2>/dev/null)"; then
    echo "unable to resolve maintainability base ref: $base_ref" >&2
    exit 2
  fi
fi

baseline_file="$repo_root/docs/maintainability-baseline.md"
if [[ ! -f "$baseline_file" ]]; then
  echo "missing maintainability baseline: $baseline_file" >&2
  exit 2
fi

is_test_path() {
  local path="$1"
  [[ "$path" == crates/*/tests/* \
    || "$path" == crates/*/benches/* \
    || "$path" == crates/*/src/tests.rs \
    || "$path" == crates/*/src/tests/* \
    || "$path" == */src/*/tests.rs \
    || "$path" == tests/* ]]
}

is_rust_source_path() {
  local path="$1"
  [[ "$path" == crates/*/src/* || "$path" == crates/*/tests/* \
    || "$path" == crates/*/benches/* \
    || "$path" == tests/* ]]
}

is_excluded_path() {
  case "$1" in
    target/*|include/recite.h|fixtures/generated/*)
      return 0
      ;;
    *)
      return 1
      ;;
  esac
}

classify_path() {
  if ! is_rust_source_path "$1" || is_excluded_path "$1"; then
    return 1
  fi
  if is_test_path "$1"; then
    printf 'test/support\n'
  else
    printf 'production\n'
  fi
}

line_count_at() {
  local revision="$1"
  local path="$2"
  if ! git -C "$repo_root" cat-file -e "${revision}:${path}" 2>/dev/null; then
    printf '0\n'
    return
  fi
  git -C "$repo_root" show "${revision}:${path}" | awk 'END { print NR + 0 }'
}

parse_baseline_rows() {
  awk -F'|' '
    function trim(value) {
      gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
      return value
    }
    /^## (Production|Test and support) surfaces$/ {
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

valid_issue_reason() {
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

declare -A baseline_seen=()
declare -A baseline_dispositions=()

validate_baseline() {
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
    if [[ ! "$path" =~ ^crates/[A-Za-z0-9._/-]+\.rs$ \
      || "$path" == */../* || "$path" == */.. \
      || "$path" == ../* || "$path" == ./* || "$path" == */./* ]]; then
      echo "invalid maintainability baseline path at line $line_number: $path" >&2
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
      cohesive|follow-up|review|exception)
        ;;
      *)
        echo "unknown baseline disposition at line $line_number: $path ($disposition)" >&2
        validation_failures=$((validation_failures + 1))
        ;;
    esac
    if ! valid_issue_reason "$disposition" "$reason"; then
      echo "malformed baseline issue/reason at line $line_number: $path" >&2
      validation_failures=$((validation_failures + 1))
    fi

    expected_kind="$(classify_path "$path" || true)"
    if [[ -z "$expected_kind" ]]; then
      echo "baseline path is outside handwritten Rust surfaces at line $line_number: $path" >&2
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
    if [[ "$expected_kind" == production ]]; then
      scrutiny=250
    else
      scrutiny=350
    fi
    actual_lines="$(line_count_at "$head_sha" "$path")"
    if [[ "$lines" =~ ^[1-9][0-9]*$ && "$lines" -ne "$actual_lines" ]]; then
      echo "baseline line count mismatch at line $line_number: $path (recorded $lines, actual $actual_lines)" >&2
      validation_failures=$((validation_failures + 1))
    fi
    if (( actual_lines <= scrutiny )); then
      echo "stale maintainability baseline row at line $line_number: $path is now $actual_lines lines" >&2
      validation_failures=$((validation_failures + 1))
    fi

    baseline_dispositions["$path"]="$disposition"
  done < <(parse_baseline_rows)

  return "$validation_failures"
}

if ! validate_baseline; then
  echo "maintainability baseline validation failed" >&2
  exit 1
fi

declare -a paths=()
if (( full_scan )); then
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <(git -C "$repo_root" ls-tree -r --name-only -z "$head_sha")
  echo "== full maintainability inventory at $head_sha =="
else
  if (( empty_base )); then
    diff_range="$base_sha $head_sha"
    diff_command=(git -C "$repo_root" diff --name-only -z --diff-filter=ACMR "$base_sha" "$head_sha" -- '*.rs')
  else
    diff_range="$base_sha...$head_sha"
    diff_command=(git -C "$repo_root" diff --name-only -z --diff-filter=ACMR "${base_sha}...${head_sha}" -- '*.rs')
  fi
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <("${diff_command[@]}")
  echo "== changed maintainability inventory: $diff_range =="
fi

failures=0
triggered=0
for path in "${paths[@]}"; do
  [[ "$path" == *.rs ]] || continue
  kind="$(classify_path "$path" || true)"
  [[ -n "$kind" ]] || continue

  if [[ "$kind" == production ]]; then
    scrutiny=250
    follow_up=400
  else
    scrutiny=350
    follow_up=500
  fi

  head_lines="$(line_count_at "$head_sha" "$path")"
  base_lines=0
  if (( ! full_scan )); then
    base_lines="$(line_count_at "$base_sha" "$path")"
  fi
  if (( head_lines <= scrutiny )); then
    continue
  fi

  triggered=$((triggered + 1))
  if [[ -z "${baseline_seen["$path"]+present}" ]]; then
    echo "missing baseline row for $path ($kind, $head_lines lines; scrutiny threshold $scrutiny)" >&2
    failures=$((failures + 1))
  fi
  if (( full_scan )); then
    echo "legacy trigger: $path ($kind, $head_lines lines; threshold $scrutiny)"
    continue
  fi

  if (( base_lines == head_lines )); then
    echo "unchanged trigger: $path ($kind, $head_lines lines; threshold $scrutiny)"
  elif (( head_lines < base_lines )); then
    echo "shrinking trigger: $path ($kind, $base_lines -> $head_lines lines)"
  else
    echo "growing trigger: $path ($kind, $base_lines -> $head_lines lines)"
  fi

  if (( head_lines > follow_up && (base_lines <= follow_up || head_lines > base_lines) )); then
    if [[ "${baseline_dispositions["$path"]-}" == exception ]]; then
      echo "documented exception: $path"
    else
      echo "follow-up threshold exceeded by new or growing $kind file: $path ($head_lines > $follow_up)" >&2
      echo "add a narrowly scoped exception with an issue and reason, or reduce the file" >&2
      failures=$((failures + 1))
    fi
  fi
done

if (( ${#paths[@]} == 0 )); then
  echo "no changed Rust source files"
fi
echo "maintainability triggers: $triggered"
if (( failures > 0 )); then
  echo "Found $failures maintainability violation(s)." >&2
  exit 1
fi
echo "Recite maintainability check passed."
