#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/check-neovim-host.sh [repo-root]

Run the installed-host Neovim evidence lane on Linux x86_64. The exact
Neovim 0.10.4 compatibility binary is downloaded to a temporary directory
from the official release asset and checksum-verified, then removed. Set
NVIM_0104_BIN to use an already downloaded exact binary. Set NVIM to select
the current pinned host (which must be 0.12.5).
EOF
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" || "${1:-}" == "help" ]]; then
  usage
  exit 0
fi
if (( $# > 1 )); then
  usage >&2
  exit 2
fi

input_root="${1:-}"
if [[ -n "$input_root" ]]; then
  repo_root="$(git -C "$input_root" rev-parse --show-toplevel)"
else
  repo_root="$(git rev-parse --show-toplevel)"
fi

if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  echo "Neovim installed-host evidence is currently implemented only for Linux x86_64" >&2
  echo "No macOS or Windows support is inferred from this lane." >&2
  exit 2
fi
for required_tool in curl sha256sum tar file; do
  if ! command -v "$required_tool" >/dev/null 2>&1; then
    echo "Neovim installed-host evidence requires $required_tool" >&2
    exit 2
  fi
done

resolve_binary() {
  local requested="$1"
  if [[ "$requested" == */* ]]; then
    [[ -x "$requested" ]] && printf '%s\n' "$requested"
  else
    command -v "$requested" || true
  fi
}

version_of() {
  "$1" --headless --clean +'qa!' 2>/dev/null || true
  "$1" --headless --version | sed -n '1s/^NVIM v//p'
}

assert_host_binary() {
  local binary="$1"
  local expected_version="$2"
  local version
  if [[ ! -x "$binary" ]]; then
    echo "Neovim host binary is not executable: $binary" >&2
    return 1
  fi
  version="$(version_of "$binary")"
  if [[ "$version" != "$expected_version" ]]; then
    echo "expected Neovim $expected_version, found ${version:-unknown} at $binary" >&2
    return 1
  fi
  if ! file "$binary" | rg -q 'ELF 64-bit.*x86-64'; then
    echo "Neovim binary is not an x86_64 Linux executable: $binary" >&2
    file "$binary" >&2 || true
    return 1
  fi
  echo "Neovim host: $version / Linux x86_64 / $binary"
}

scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-neovim-host.XXXXXX")"
cleanup() {
  rm -rf "$scratch"
}
trap cleanup EXIT

minimum_binary="$(resolve_binary "${NVIM_0104_BIN:-}")"
if [[ -z "$minimum_binary" ]]; then
  archive="$scratch/nvim-linux-x86_64.tar.gz"
  release_url="https://github.com/neovim/neovim/releases/download/v0.10.4/nvim-linux-x86_64.tar.gz"
  expected_sha256="95aaa8e89473f5421114f2787c13ae0ec6e11ebbd1a13a1bd6fcf63420f8073f"
  echo "Downloading official Neovim 0.10.4 Linux x86_64 release asset"
  curl --fail --location --silent --show-error --retry 2 --output "$archive" "$release_url"
  printf '%s  %s\n' "$expected_sha256" "$archive" | sha256sum --check --status
  tar -xzf "$archive" -C "$scratch"
  minimum_binary="$scratch/nvim-linux-x86_64/bin/nvim"
  echo "Verified official asset: $release_url"
  echo "Verified SHA-256: $expected_sha256"
else
  echo "Using caller-provided NVIM_0104_BIN; version and ELF checks remain enforced"
fi
assert_host_binary "$minimum_binary" "0.10.4"

current_binary="$(resolve_binary "${NVIM:-nvim}")"
if [[ -z "$current_binary" ]]; then
  echo "Current pinned Neovim 0.12.5 is unavailable; set NVIM=/path/to/nvim" >&2
  exit 2
fi
assert_host_binary "$current_binary" "0.12.5"

run_lane() {
  local label="$1"
  local binary="$2"
  echo "== Neovim $label installed-host evidence =="
  NVIM="$binary" "$repo_root/scripts/check-neovim.sh" --host-evidence "$repo_root"
}

run_lane "0.10.4 compatibility" "$minimum_binary"
run_lane "0.12.5 current" "$current_binary"
echo "Neovim installed-host evidence passed for 0.10.4 and 0.12.5 on Linux x86_64"
