set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Provision only the pinned release tools in addition to the checkout toolchain.
setup:
    mise -E release install github:crate-ci/cargo-release aqua:axodotdev/cargo-dist

# Validate the CLI/LSP distribution graph without constructing release binaries.
plan:
    mise -E release exec -- dist plan

# Preview the exact version changes; this does not commit, tag, push or publish.
prepare version:
    cargo run --locked -p recite-release -- version "$1"
    mise -E release exec -- cargo release version "$1" --workspace --exclude recite-ffi --exclude recite-release --isolated --config release.toml
    mise -E release exec -- cargo release version "$1" --manifest-path apps/writer/Cargo.toml --workspace --isolated --config "$PWD/release.toml"

# Apply reviewed version changes on a release branch; Git remains maintainer-owned.
set-version version:
    cargo run --locked -p recite-release -- version "$1"
    mise -E release exec -- cargo release version "$1" --workspace --exclude recite-ffi --exclude recite-release --isolated --config release.toml --execute --no-confirm
    mise -E release exec -- cargo release version "$1" --manifest-path apps/writer/Cargo.toml --workspace --isolated --config "$PWD/release.toml" --execute --no-confirm
    mise -E release exec -- cargo release replace --manifest-path apps/writer/Cargo.toml --package recite-writer --isolated --config "$PWD/apps/writer/packaging/release.toml" --execute --no-confirm
    mise -E quality exec -- dprint fmt apps/writer/packaging/common.json

# Produce local CLI/LSP artifacts using cargo-dist's archive and checksum machinery.
build target:
    mkdir -p target/distrib
    mise -E release exec -- dist build --artifacts=local --target "$1" --output-format=json > "target/$1-dist-manifest.json"
    mv "target/$1-dist-manifest.json" target/distrib/
