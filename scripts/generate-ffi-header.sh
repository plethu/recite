#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  generate-ffi-header.sh [--write] [repo-root]

Verifies that include/recite.h is up to date with crates/recite-ffi.
Pass --write to regenerate the committed header.
EOF
}

mode="verify"
cbindgen_version="0.29.4"
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
elif [[ "${1:-}" == "--write" ]]; then
  mode="write"
  shift
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  if ! repo_root="$(git -C "$input_root" rev-parse --show-toplevel 2>/dev/null)"; then
    echo "repo root is not a git checkout: $input_root" >&2
    exit 2
  fi
else
  if ! repo_root="$(git rev-parse --show-toplevel 2>/dev/null)"; then
    echo "unable to resolve git repo root from current directory" >&2
    exit 2
  fi
fi

if ! command -v cbindgen >/dev/null 2>&1; then
  cat >&2 <<'EOF'
missing required tool: cbindgen

Install it with:
  cargo install cbindgen --version 0.29.4 --locked
EOF
  exit 2
fi

installed_cbindgen_version="$(cbindgen --version | awk '{ print $2 }')"
if [[ "$installed_cbindgen_version" != "$cbindgen_version" ]]; then
  echo "cbindgen $cbindgen_version is required; found $installed_cbindgen_version" >&2
  echo "install it with: cargo install cbindgen --version $cbindgen_version --locked" >&2
  exit 2
fi

header="$repo_root/include/recite.h"
mkdir -p "$repo_root/include"

crate_version="$(awk -F ' *= *' '
  /^version[[:space:]]*=/ {
    gsub(/"/, "", $2)
    print $2
    exit
  }
  /^version\.workspace[[:space:]]*=/ {
    workspace = 1
  }
  END {
    if (workspace) {
      exit 42
    }
  }
' "$repo_root/crates/recite-ffi/Cargo.toml")" || {
  status="$?"
  if [[ "$status" != "42" ]]; then
    exit "$status"
  fi
  crate_version="$(awk -F ' *= *' '
    /^\[workspace\.package\]/ {
      in_workspace_package = 1
      next
    }
    /^\[/ {
      in_workspace_package = 0
    }
    in_workspace_package && /^version[[:space:]]*=/ {
      gsub(/"/, "", $2)
      print $2
      exit
    }
  ' "$repo_root/Cargo.toml")"
}
IFS=. read -r crate_major crate_minor crate_patch extra_version <<<"$crate_version"
if [[ -z "${crate_major:-}" || -z "${crate_minor:-}" || -z "${crate_patch:-}" || -n "${extra_version:-}" ]]; then
  echo "unable to parse recite-ffi crate version: $crate_version" >&2
  exit 2
fi

cbindgen_args=(
  "$repo_root/crates/recite-ffi"
  --config "$repo_root/cbindgen.toml"
  --lockfile "$repo_root/Cargo.lock"
  --quiet
  --output "$header"
)

if [[ "$mode" == "write" ]]; then
  cbindgen "${cbindgen_args[@]}"
else
  if [[ ! -f "$header" ]]; then
    echo "missing generated header: $header" >&2
    echo "run scripts/generate-ffi-header.sh --write" >&2
    exit 1
  fi
  cbindgen "${cbindgen_args[@]}" --verify
fi

check_macro() {
  local name="$1"
  local expected="$2"
  local actual

  actual="$(awk -v name="$name" '$1 == "#define" && $2 == name { print $3; exit }' "$header")"
  if [[ "$actual" != "$expected" ]]; then
    echo "header version macro $name=$actual does not match recite-ffi crate version $crate_version" >&2
    exit 1
  fi
}

check_macro RECITE_FFI_VERSION_MAJOR "$crate_major"
check_macro RECITE_FFI_VERSION_MINOR "$crate_minor"
check_macro RECITE_FFI_VERSION_PATCH "$crate_patch"

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

rg -o 'pub (unsafe )?extern "C" fn recite_[A-Za-z0-9_]+' "$repo_root/crates/recite-ffi/src" -N \
  | awk '{ print $NF }' \
  | sort -u >"$tmpdir/rust-symbols"

rg -o '\brecite_[A-Za-z0-9_]+\(' "$header" -N \
  | sed 's/($//' \
  | sort -u >"$tmpdir/header-symbols"

if ! diff -u "$tmpdir/rust-symbols" "$tmpdir/header-symbols" >/dev/null; then
  echo "generated header does not cover every recite-ffi extern C symbol" >&2
  diff -u "$tmpdir/rust-symbols" "$tmpdir/header-symbols" >&2 || true
  exit 1
fi
