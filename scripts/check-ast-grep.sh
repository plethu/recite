#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-ast-grep.sh [base-ref [head-ref]] [--full]

Runs Recite's pinned ast-grep fixture tests and scans changed handwritten Rust
in crate source and benchmark targets. Use --full to scan all tracked source
and benchmark Rust.
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
config="$repo_root/tools/ast-grep/sgconfig.yml"
if [[ ! -f "$config" ]]; then
  echo "missing ast-grep config: $config" >&2
  exit 2
fi
if ! command -v ast-grep >/dev/null 2>&1; then
  echo "missing required tool: ast-grep" >&2
  echo "Run the scoped check with: mise -E maintainability exec -- scripts/check-ast-grep.sh" >&2
  exit 2
fi

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
empty_base=0
if ! head_sha="$(git -C "$repo_root" rev-parse --verify "${head_ref}^{commit}" 2>/dev/null)"; then
  echo "unable to resolve ast-grep head ref: $head_ref" >&2
  exit 2
fi

echo "== ast-grep fixture tests =="
(
  cd "$repo_root"
  ast-grep test --config tools/ast-grep/sgconfig.yml --skip-snapshot-tests
)

declare -a paths=()
if (( full_scan )); then
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <(git -C "$repo_root" ls-tree -r --name-only -z "$head_sha")
else
  if [[ "$base_ref" =~ ^0{40}$ ]]; then
    base_sha="$(git -C "$repo_root" hash-object -t tree /dev/null)"
    empty_base=1
    echo "zero/initial base SHA; using the empty tree $base_sha"
  elif ! base_sha="$(git -C "$repo_root" rev-parse --verify "${base_ref}^{commit}" 2>/dev/null)"; then
    echo "unable to resolve ast-grep base ref: $base_ref" >&2
    exit 2
  fi
  if (( empty_base )); then
    diff_command=(git -C "$repo_root" diff --name-only -z --diff-filter=ACMR "$base_sha" "$head_sha")
  else
    diff_command=(git -C "$repo_root" diff --name-only -z --diff-filter=ACMR "${base_sha}...${head_sha}")
  fi
  while IFS= read -r -d '' path; do
    paths+=("$path")
  done < <("${diff_command[@]}")
fi

declare -a scan_paths=()
for path in "${paths[@]}"; do
  [[ "$path" == *.rs ]] || continue
  if [[ "$path" != crates/*/src/* && "$path" != crates/*/benches/* ]]; then
    continue
  fi
  if [[ "$path" == crates/*/src/tests.rs || "$path" == crates/*/src/tests/* || "$path" == */src/*/tests.rs ]]; then
    continue
  fi
  case "$path" in
    target/*|fixtures/generated/*)
      continue
      ;;
  esac
  scan_paths+=("$path")
done

if (( ${#scan_paths[@]} == 0 )); then
  echo "no changed Rust source files to scan"
  exit 0
fi

echo "== ast-grep structural scan =="
(
  cd "$repo_root"
  ast-grep scan --config tools/ast-grep/sgconfig.yml "${scan_paths[@]}"
)
