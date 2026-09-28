#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target_dir="${CARGO_TARGET_DIR:-$repo_root/target}"
artifact_dir="$target_dir/recite-bevy-probe/cargo-packages"
scratch="$(mktemp -d "${TMPDIR:-/tmp}/recite-bevy-package.XXXXXX")"
trap 'rm -rf "$scratch"' EXIT
stage="$scratch/artifacts"
package_version="$(python3 - "$repo_root/Cargo.toml" <<'PY'
import pathlib
import sys
import tomllib
print(tomllib.loads(pathlib.Path(sys.argv[1]).read_text())["workspace"]["package"]["version"])
PY
)"

crates=(recite-core recite-parser recite-compiler recite-runtime recite-adapter recite-ui recite-bevy)
patches=()
for crate in "${crates[@]}"; do
    patches+=(--config "patch.crates-io.$crate.path=\"$repo_root/crates/$crate\"")
done

mkdir -p "$stage/packages" "$scratch/unpacked" "$scratch/consumer/examples/assets"
cp "$repo_root/LICENSE-MIT" "$repo_root/LICENSE-APACHE" "$stage/"
test -s "$stage/LICENSE-MIT"
test -s "$stage/LICENSE-APACHE"

# Temporary patches resolve unpublished Recite versions while Cargo prepares
# ordinary registry archives with normalized dependency manifests.
for crate in "${crates[@]}"; do
    CARGO_TARGET_DIR="$target_dir" cargo package --manifest-path "$repo_root/Cargo.toml" \
        -p "$crate" --allow-dirty --no-verify --offline "${patches[@]}"
    cp "$target_dir/package/$crate-$package_version.crate" "$stage/packages/"
done

# Preparing the same inputs a second time must produce byte-identical archives.
for crate in "${crates[@]}"; do
    CARGO_TARGET_DIR="$target_dir" cargo package --manifest-path "$repo_root/Cargo.toml" \
        -p "$crate" --allow-dirty --no-verify --offline "${patches[@]}"
    cmp "$stage/packages/$crate-$package_version.crate" "$target_dir/package/$crate-$package_version.crate"
done

python3 - "$stage/packages" "$scratch/unpacked" "$package_version" <<'PY'
import hashlib
import pathlib
import sys
import tarfile
import tomllib

packages = pathlib.Path(sys.argv[1])
unpacked = pathlib.Path(sys.argv[2])
version = sys.argv[3]
names = ("recite-core", "recite-parser", "recite-compiler", "recite-runtime",
         "recite-adapter", "recite-ui", "recite-bevy")
lines = []
for name in names:
    archive = packages / f"{name}-{version}.crate"
    prefix = f"{name}-{version}/"
    with tarfile.open(archive, "r:gz") as tar:
        members = tar.getmembers()
        paths = {member.name for member in members}
        assert f"{prefix}Cargo.toml" in paths, archive
        assert f"{prefix}src/lib.rs" in paths, archive
        assert all(member.name.startswith(prefix) and member.isfile()
                   and member.uid == 0 and member.gid == 0
                   and member.mtime == members[0].mtime for member in members), archive
        if name == "recite-bevy":
            assert f"{prefix}examples/headless_dialogue.rs" in paths
            assert f"{prefix}examples/assets/demo.recite" in paths
            assert f"{prefix}README.md" in paths
        manifest = tomllib.loads(tar.extractfile(f"{prefix}Cargo.toml").read().decode())
        assert manifest["package"]["license"] == "MIT OR Apache-2.0", archive
        for kind in ("dependencies", "dev-dependencies", "build-dependencies"):
            for dependency, specification in manifest.get(kind, {}).items():
                if dependency.startswith("recite-"):
                    assert "path" not in specification, (archive, dependency)
        tar.extractall(unpacked, filter="data")
    lines.append(f"{hashlib.sha256(archive.read_bytes()).hexdigest()}  {archive.name}")
(packages.parent / "SHA256SUMS").write_text("\n".join(lines) + "\n")
PY

# Compile the example shipped inside the Bevy archive as an external project.
# Every Recite dependency below points to an extracted .crate.
bevy_archive="$scratch/unpacked/recite-bevy-$package_version"
cp "$bevy_archive/examples/headless_dialogue.rs" "$scratch/consumer/examples/"
cp "$bevy_archive/examples/assets/demo.recite" "$scratch/consumer/examples/assets/"
cat > "$scratch/consumer/Cargo.toml" <<EOF
[package]
name = "recite-bevy-package-consumer"
version = "0.0.0"
edition = "2024"

[dependencies]
bevy_app = { version = "=0.19.1", default-features = false, features = ["std"] }
bevy_asset = { version = "=0.19.1", default-features = false }
bevy_ecs = { version = "=0.19.1", default-features = false, features = ["std"] }
recite-bevy = "=$package_version"
recite-compiler = "=$package_version"
recite-core = "=$package_version"
recite-runtime = "=$package_version"
EOF
for crate in "${crates[@]}"; do
    if [[ "$crate" == "recite-core" ]]; then
        printf '\n[patch.crates-io]\n' >> "$scratch/consumer/Cargo.toml"
    fi
    printf '%s = { path = "../unpacked/%s-%s" }\n' "$crate" "$crate" "$package_version" >> "$scratch/consumer/Cargo.toml"
done
CARGO_TARGET_DIR="$target_dir" cargo run --manifest-path "$scratch/consumer/Cargo.toml" \
    --example headless_dialogue --offline
CARGO_TARGET_DIR="$target_dir" cargo tree --manifest-path "$scratch/consumer/Cargo.toml" \
    --offline > "$stage/dependency-tree.txt"
for crate in "${crates[@]}"; do
    [[ "$crate" == "recite-ui" ]] && continue # only an unpublished dev dependency
    rg -Fq "$crate v$package_version ($scratch/unpacked/$crate-$package_version)" "$stage/dependency-tree.txt"
done
if rg -q 'bevy_render|wgpu|winit|bevy_window' "$stage/dependency-tree.txt"; then
    echo "renderer/window dependency leaked into CPU-only adapter" >&2
    exit 1
fi
mv "$scratch/unpacked" "$scratch/consumer" "$stage/"
mkdir -p "$(dirname "$artifact_dir")"
if [[ -e "$artifact_dir" ]]; then
    mv "$artifact_dir" "$scratch/previous-artifacts"
fi
if ! mv "$stage" "$artifact_dir"; then
    if [[ -e "$scratch/previous-artifacts" ]]; then
        mv "$scratch/previous-artifacts" "$artifact_dir"
    fi
    exit 1
fi
printf 'Bevy Cargo packages and packaged example passed; archives: %s\n' "$artifact_dir/packages"
