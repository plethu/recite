#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-maintainability.sh [base-ref [head-ref]] [--full]

Checks changed handwritten Rust, JavaScript, Lua, Python, and shell source.
Line counts are review triggers, not automatic split rules:
  production and tooling: scrutiny >250, follow-up >400
  test/support: scrutiny >350, follow-up >500

Unchanged or shrinking oversized files pass. A file crossing or growing above
its follow-up threshold requires an exact, bounded, issue-linked exception in
scripts/maintainability/exceptions.toml. File sizes are read from the checked
out head; use --full for a repository-wide trigger report.
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

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# Keep policy functions separate from this orchestration entrypoint.  This
# prevents the gate from becoming an oversized tooling file as languages are
# added while keeping the policy executable and fixture-testable.
# shellcheck source=scripts/maintainability/paths.sh
source "$script_dir/maintainability/paths.sh"
# shellcheck source=scripts/maintainability/exceptions.sh
source "$script_dir/maintainability/exceptions.sh"
# shellcheck source=scripts/maintainability/diff.sh
source "$script_dir/maintainability/diff.sh"

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

exceptions_file="$repo_root/scripts/maintainability/exceptions.toml"
if [[ ! -f "$exceptions_file" ]]; then
  echo "missing maintainability exceptions: $exceptions_file" >&2
  exit 2
fi

declare -A exception_maximum=()
if ! maintainability_validate_exceptions; then
  echo "maintainability exception validation failed" >&2
  exit 1
fi

declare -a all_paths=()
while IFS= read -r -d '' path; do
  all_paths+=("$path")
done < <(git -C "$repo_root" ls-tree -r --name-only -z "$head_sha")

declare -a paths=()
declare -A base_paths=()
declare -A renamed_paths=()
if (( full_scan )); then
  paths=("${all_paths[@]}")
  echo "== full maintainability inventory at $head_sha =="
else
  diff_range=""
  if ! maintainability_collect_paths "$repo_root" "$base_sha" "$head_sha" "$empty_base" paths base_paths renamed_paths; then
    exit 2
  fi
  echo "== changed maintainability inventory: $diff_range =="
fi

failures=0
triggered=0

for path in "${all_paths[@]}"; do
  kind="$(maintainability_classify_path "$path" || true)"
  [[ -n "$kind" ]] || continue
  if ! maintainability_is_regular_file_at "$repo_root" "$head_sha" "$path"; then
    echo "eligible maintainability source is not a regular file: $path" >&2
    failures=$((failures + 1))
  fi
done

for path in "${paths[@]}"; do
  kind="$(maintainability_classify_path "$path" || true)"
  [[ -n "$kind" ]] || continue

  scrutiny="$(maintainability_scrutiny_threshold "$kind")"
  follow_up="$(maintainability_follow_up_threshold "$kind")"
  head_lines="$(maintainability_line_count_at "$repo_root" "$head_sha" "$path")"
  base_lines=0
  policy_transition=0
  if (( ! full_scan )); then
    base_path="${base_paths["$path"]-$path}"
    base_lines="$(maintainability_line_count_at "$repo_root" "$base_sha" "$base_path")"
    if [[ -n "${renamed_paths["$path"]+present}" ]]; then
      base_kind="$(maintainability_classify_path "$base_path" || true)"
      if [[ -z "$base_kind" ]]; then
        policy_transition=1
      else
        base_scrutiny="$(maintainability_scrutiny_threshold "$base_kind")"
        base_follow_up="$(maintainability_follow_up_threshold "$base_kind")"
        if (( scrutiny < base_scrutiny || follow_up < base_follow_up )); then
          policy_transition=1
        fi
      fi
      if (( policy_transition )); then
        base_lines=0
      fi
    fi
  fi
  if (( head_lines <= scrutiny )); then
    continue
  fi

  triggered=$((triggered + 1))
  if (( full_scan )); then
    echo "legacy trigger: $path ($kind, $head_lines lines; threshold $scrutiny)"
    continue
  fi

  if (( policy_transition )); then
    echo "policy transition trigger: $path ($kind, newly entering stricter maintainability policy; threshold $scrutiny)"
  elif (( base_lines == head_lines )); then
    echo "unchanged trigger: $path ($kind, $head_lines lines; threshold $scrutiny)"
  elif (( head_lines < base_lines )); then
    echo "shrinking trigger: $path ($kind, $base_lines -> $head_lines lines)"
  else
    echo "growing trigger: $path ($kind, $base_lines -> $head_lines lines)"
  fi

  if (( head_lines > follow_up && (base_lines <= follow_up || head_lines > base_lines) )); then
    if [[ -n "${exception_maximum["$path"]+present}" ]]; then
      echo "documented exception: $path"
    else
      echo "follow-up threshold exceeded by new or growing $kind file: $path ($head_lines > $follow_up)" >&2
      echo "add a narrowly scoped exception with an issue and reason, or reduce the file" >&2
      failures=$((failures + 1))
    fi
  fi
done

if (( ${#paths[@]} == 0 )); then
  echo "no changed maintainability source files"
fi
echo "maintainability triggers: $triggered"
if (( failures > 0 )); then
  echo "Found $failures maintainability violation(s)." >&2
  exit 1
fi
echo "Recite maintainability check passed."
