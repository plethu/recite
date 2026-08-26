#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-project-gates.sh [repo-root]

Runs Recite's Rust and adapter project gates (the full local suite is
scripts/verify.sh or `mise run verify`):
  1. scripts/check-test-organization.sh
  2. scripts/generate-ffi-header.sh
  3. scripts/check-unity-adapter.sh
  4. cargo fmt --check
  5. cargo test --locked
  6. cargo clippy --locked --all-targets --all-features -- -D warnings
  7. RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
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

if [[ ! -x "$repo_root/scripts/check-test-organization.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-test-organization.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/generate-ffi-header.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/generate-ffi-header.sh" >&2
  exit 2
fi

if [[ -e "$repo_root/scripts/check-unity-adapter.sh" && ! -x "$repo_root/scripts/check-unity-adapter.sh" ]]; then
  echo "non-executable gate: $repo_root/scripts/check-unity-adapter.sh" >&2
  exit 2
fi

echo "== test organization =="
"$repo_root/scripts/check-test-organization.sh" "$repo_root"

echo
echo "== generated ffi header =="
"$repo_root/scripts/generate-ffi-header.sh" "$repo_root"

if [[ -x "$repo_root/scripts/check-unity-adapter.sh" ]]; then
  echo
  echo "== unity adapter package =="
  "$repo_root/scripts/check-unity-adapter.sh" "$repo_root"
fi

echo
echo "== cargo fmt --check =="
(
  cd "$repo_root"
  cargo fmt --check
)

echo
echo "== cargo test =="
(
  cd "$repo_root"
  cargo test --locked
)

echo
echo "== cargo clippy =="
(
  cd "$repo_root"
  cargo clippy --locked --all-targets --all-features -- -D warnings
)

echo
echo "== cargo doc =="
(
  cd "$repo_root"
  RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
)

echo
echo "Recite project gates passed."
