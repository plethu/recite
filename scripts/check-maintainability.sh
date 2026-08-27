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
reason. Use --full for a repository-wide baseline inventory.
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
if (( ! full_scan )); then
  if ! base_sha="$(git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" 2>/dev/null)"; then
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
    || "$path" == crates/*/src/tests.rs \
    || "$path" == crates/*/src/tests/* \
    || "$path" == */src/*/tests.rs \
    || "$path" == tests/* ]]
}

is_rust_source_path() {
  local path="$1"
  [[ "$path" == crates/*/src/* || "$path" == crates/*/tests/* \
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
    printf 'test\n'
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

baseline_entry() {
  local path="$1"
  awk -F'|' -v needle="\`$path\`" '
    index($2, needle) { print; exit }
  ' "$baseline_file"
}

has_valid_exception() {
  local entry="$1"
  [[ "$entry" == *"| exception |"* \
    && "$entry" =~ \#[1-9][0-9]* \
    && "$entry" != *"| exception | |"* ]]
}

declare -a paths=()
if (( full_scan )); then
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <(git -C "$repo_root" ls-tree -r --name-only -z "$head_sha")
  echo "== full maintainability inventory at $head_sha =="
else
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <(git -C "$repo_root" diff --name-only -z --diff-filter=ACMR "${base_sha}...${head_sha}" -- '*.rs')
  echo "== changed maintainability inventory: $base_sha...$head_sha =="
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
  entry="$(baseline_entry "$path" || true)"
  if [[ -z "$entry" ]]; then
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
    if has_valid_exception "$entry"; then
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
