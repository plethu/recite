#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/.." && pwd -P)
manifest="$repo/apps/writer/packaging/flatpak/io.github.plethu.recite.yml"
output=${1:-"$repo/target/writer-flatpak"}
output=$(realpath -m "$output")

# The manifest copies this checkout as a source. Only target/ is excluded.
if [[ "$output" == / || "$repo" == "$output" || "$repo" == "$output/"* ]] ||
   [[ "$output" == "$repo/"* && "$output" != "$repo/target" && "$output" != "$repo/target/"* ]]; then
    echo 'output must be under target/ or outside the checkout and its ancestors' >&2
    exit 2
fi
mkdir -p "$output"
output=$(cd "$output" && pwd -P)

if ! command -v flatpak-builder >/dev/null; then
    echo 'flatpak-builder is required' >&2
    exit 1
fi

# The SDK and Rust extension must already be installed in the selected Flatpak
# installation. This script only builds and exports an artifact.
flatpak-builder \
    --disable-rofiles-fuse \
    --force-clean \
    --state-dir="$output/state" \
    --repo="$output/repo" \
    "$output/build" "$manifest"
flatpak build-bundle \
    "$output/repo" \
    "$output/io.github.plethu.recite.flatpak" \
    io.github.plethu.recite
(cd "$output" && sha256sum io.github.plethu.recite.flatpak > SHA256SUMS)
python3 "$repo/scripts/check-writer-flatpak.py" "$output"
