#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  check-project-gates.sh [repo-root]

Runs Recite's Rust and adapter project gates (the full local suite is
scripts/verify.sh or `mise run verify`):
  1. scripts/check-test-organization.sh
  2. scripts/check-tree-sitter.sh
  3. scripts/check-neovim.sh
  4. scripts/check-zed.sh
  5. scripts/check-editor-parity.sh
  6. scripts/check-lint-suppressions.sh
  7. scripts/generate-ffi-header.sh
  8. scripts/check-ffi-header.sh
  9. scripts/check-unity-adapter.sh (managed/native; Unity Editor only with UNITY_EDITOR)
 10. just test-godot (clean addon and native host)
 11. cargo fetch --locked; scripts/check-bevy-package.sh (offline clean consumer)
 12. cargo fmt --check
 13. just test and just test-doc
 14. just clippy
 15. RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
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

if [[ ! -x "$repo_root/scripts/check-lint-suppressions.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-lint-suppressions.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/check-editor-parity.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-editor-parity.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/check-tree-sitter.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-tree-sitter.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/check-neovim.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-neovim.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/check-zed.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-zed.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/generate-ffi-header.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/generate-ffi-header.sh" >&2
  exit 2
fi

if [[ ! -x "$repo_root/scripts/check-ffi-header.sh" ]]; then
  echo "missing executable gate: $repo_root/scripts/check-ffi-header.sh" >&2
  exit 2
fi

for gate in check-unity-adapter.sh check-godot-host.sh package-godot-addon.sh check-bevy-package.sh; do
  if [[ ! -x "$repo_root/scripts/$gate" ]]; then
    echo "missing executable adapter gate: $repo_root/scripts/$gate" >&2
    exit 2
  fi
done

echo "== test organization =="
"$repo_root/scripts/check-test-organization.sh" "$repo_root"

echo
echo "== editor parity contract =="
"$repo_root/scripts/check-editor-parity.sh" "$repo_root"

echo
echo "== Tree-sitter grammar =="
"$repo_root/scripts/check-tree-sitter.sh" "$repo_root"

echo
echo "== Neovim integration =="
"$repo_root/scripts/check-neovim.sh" "$repo_root"

echo
echo "== Zed extension package =="
"$repo_root/scripts/check-zed.sh" "$repo_root"

echo
echo "== lint suppression policy =="
(
  cd "$repo_root"
  scripts/check-lint-suppressions.sh
)

echo
echo "== generated ffi header =="
"$repo_root/scripts/generate-ffi-header.sh" "$repo_root"

echo
echo "== ffi header C/C++ probes =="
"$repo_root/scripts/check-ffi-header.sh" "$repo_root"

echo
echo "== Unity managed/native adapter package =="
"$repo_root/scripts/check-unity-adapter.sh" "$repo_root"

echo
echo "== Godot clean addon and native host =="
(
  cd "$repo_root"
  just test-godot
)

echo
echo "== Bevy real App and clean package consumer =="
(
  cd "$repo_root"
  cargo fetch --locked
)
"$repo_root/scripts/check-bevy-package.sh"

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
  just test-workflow
  just test
  just test-doc
)

echo
echo "== cargo clippy =="
(
  cd "$repo_root"
  just clippy
)

echo
echo "== cargo doc =="
(
  cd "$repo_root"
  RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps
)

echo
(cd "$repo_root" && just check-writer)

echo "Recite project gates passed."
