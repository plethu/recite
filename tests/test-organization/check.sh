#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
scratch="$(mktemp -d)"
trap 'rm -rf "$scratch"' EXIT
git -C "$scratch" init -q
mkdir -p "$scratch/crates/demo/src"
source_file="$scratch/crates/demo/src/lib.rs"

cat >"$source_file" <<'RUST'
mod private_owner {
    #[cfg(test)]
    mod tests;
}
mod parent_support {
    #[cfg(test)]
    pub(super) mod tests;
}
mod crate_support {
    #[cfg(test)]
    pub(crate) mod tests;
}
RUST
"$repo_root/scripts/check-test-organization.sh" "$scratch"

reject() {
  local name="$1" expected="$2"
  cat >"$source_file"
  if "$repo_root/scripts/check-test-organization.sh" "$scratch" >"$scratch/result" 2>&1; then
    echo "test organization accepted $name" >&2
    exit 1
  fi
  if ! grep -Fq "$expected" "$scratch/result"; then
    cat "$scratch/result" >&2
    echo "wrong rejection for $name" >&2
    exit 1
  fi
}
reject reexport 'external tests sidecar' <<'RUST'
#[cfg(test)]
pub(crate) use private::Driver;
RUST
reject implementation 'external tests sidecar' <<'RUST'
#[cfg(test)]
impl Owner {}
RUST
reject unrestricted-sidecar 'external tests sidecar' <<'RUST'
#[cfg(test)]
pub mod tests;
RUST
for visibility in '' 'pub(super) ' 'pub(crate) ' 'pub '; do
  reject "${visibility}inline" 'inline mod tests blocks' <<<"${visibility}mod tests {}"
done
reject production-test 'belongs in tests.rs' <<'RUST'
#[test]
fn misplaced_test() {}
RUST
echo 'Test organization boundary fixtures passed.'
