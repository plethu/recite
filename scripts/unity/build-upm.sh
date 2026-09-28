#!/usr/bin/env bash
set -euo pipefail
repo_root="${1:-$(git rev-parse --show-toplevel)}"
package_dir="$repo_root/Packages/com.recite.dialogue"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target/companions-unity}"
version="$(python3 - "$package_dir/package.json" <<'PY'
import json,sys
print(json.load(open(sys.argv[1]))['version'])
PY
)"
case "$(uname -s)-$(uname -m)" in
  Linux-x86_64) platform=linux-x86_64; library=librecite_ffi.so ;;
  *) echo 'This bundle builder currently supports Linux x86_64 only.' >&2; exit 2 ;;
esac
CARGO_TARGET_DIR="$target_dir" cargo build --manifest-path "$repo_root/Cargo.toml" -p recite-ffi --release
native="$target_dir/release/$library"
[[ -f "$native" ]] || { echo "missing built native library: $native" >&2; exit 1; }
nm -D "$native" | rg -q ' recite_asset_info$' || { echo 'native library lacks asset-info ABI' >&2; exit 1; }
work="$(mktemp -d /tmp/recite-unity-bundle.XXXXXX)"
trap 'rm -rf "$work"' EXIT
mkdir -p "$work/package/Runtime/Plugins/x86_64"
cp -a "$package_dir/." "$work/package/"
cp "$native" "$work/package/Runtime/Plugins/x86_64/$library"
cp "$repo_root/LICENSE-MIT" "$repo_root/LICENSE-APACHE" "$work/package/"
cat > "$work/package/Runtime/Plugins/x86_64/$library.meta" <<'META'
fileFormatVersion: 2
guid: 08d0308e5291518db21b4f65c55aa916
PluginImporter:
  externalObjects: {}
  serializedVersion: 2
  iconMap: {}
  executionOrder: {}
  defineConstraints: []
  isPreloaded: 0
  isOverridable: 0
  isExplicitlyReferenced: 0
  validateReferences: 1
  platformData:
  - first:
      Any:
    second:
      enabled: 0
      settings: {}
  - first:
      Editor: Editor
    second:
      enabled: 1
      settings:
        CPU: x86_64
        OS: Linux
  - first:
      Standalone: Linux64
    second:
      enabled: 1
      settings:
        CPU: x86_64
  userData:
  assetBundleName:
  assetBundleVariant:
META
mkdir -p "$target_dir/bundles"
out="$target_dir/bundles/com.recite.dialogue-$version-$platform.tgz"
tar -C "$work" --sort=name --mtime='UTC 1970-01-01' --owner=0 --group=0 --numeric-owner -cf - package | gzip -n > "$out"
python3 - "$out" <<'PY'
import json,sys,tarfile
with tarfile.open(sys.argv[1], 'r:gz') as archive:
    names=set(archive.getnames())
    required={'package/package.json','package/Editor/ReciteCompiledImporter.cs',
              'package/Runtime/Plugins/x86_64/librecite_ffi.so',
              'package/Runtime/Plugins/x86_64/librecite_ffi.so.meta',
              'package/Tests/Editor/ReciteImporterTests.cs',
              'package/Tests/Runtime/RecitePlayModeTests.cs'}
    assert required <= names, required-names
    manifest=json.load(archive.extractfile('package/package.json'))
    assert manifest['unity']=='2022.3'
print('UPM bundle contents passed.')
PY
printf '%s\n' "$out"
