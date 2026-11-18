#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
destination="${1:-$repo_root/target/godot-addon}"
profile="${RECITE_GODOT_PROFILE:-release}"
case "$profile" in
  release) cargo_profile=(--release) ;;
  debug) cargo_profile=() ;;
  *)
    echo "RECITE_GODOT_PROFILE must be release or debug" >&2
    exit 2
    ;;
esac
if [[ -n "${CARGO_TARGET_DIR:-}" && "$CARGO_TARGET_DIR" != /* ]]; then
  cargo_target_dir="$repo_root/$CARGO_TARGET_DIR"
else
  cargo_target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
fi
export CARGO_TARGET_DIR="$cargo_target_dir"
mkdir -p "$destination"
marker="$destination/.recite-godot-package"
if [[ ! -f "$marker" && (-e "$destination/addons/recite" || -e "$destination/examples/basic-dialogue") ]]; then
  echo "Refusing to replace an unmarked addon or example at $destination; use a dedicated package output directory." >&2
  exit 2
fi

cargo build --locked -p recite-godot --manifest-path "$repo_root/Cargo.toml" "${cargo_profile[@]}"

stage="$(mktemp -d "${TMPDIR:-/tmp}/recite-godot-package.XXXXXX")"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/addons/recite/bin" "$stage/examples/basic-dialogue"
cp -R "$repo_root/addons/recite/." "$stage/addons/recite/"
tar -C "$repo_root/examples/godot/basic-dialogue" \
  --exclude='./.godot' --exclude='*.recitec' -cf - . \
  | tar -C "$stage/examples/basic-dialogue" -xf -
cp "$cargo_target_dir/$profile/librecite_godot.so" "$stage/addons/recite/bin/"
cp "$repo_root/LICENSE-MIT" "$repo_root/LICENSE-APACHE" "$stage/addons/recite/"

# The output directory may contain a consumer project. Replace only paths
# owned by this package, and never touch its authored dialogue or scenes.
rm -rf "$destination/addons/recite" "$destination/examples/basic-dialogue"
mkdir -p "$destination/addons" "$destination/examples"
cp -R "$stage/addons/recite" "$destination/addons/"
cp -R "$stage/examples/basic-dialogue" "$destination/examples/"
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
  --format=ustar -C "$stage" -cf - addons examples | gzip -n >"$destination/recite-godot-addon.tar.gz"
printf 'Recite Godot package output; only addons/recite and examples/basic-dialogue are replaced.\n' >"$marker"
echo "$destination/addons/recite"
