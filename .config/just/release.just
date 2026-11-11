set working-directory := '../..'
set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Provision only the pinned release tools in addition to the checkout toolchain.
setup:
    mise -E release install github:crate-ci/cargo-release aqua:axodotdev/cargo-dist github:obi1kenobi/cargo-semver-checks

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

# `just supply-chain` fails on unmaintained direct dependencies only; record these in the release issue.
# List unmaintained advisories anywhere in the root, Zed and Writer graphs; fails while any remain.
dependency-review:
    #!/usr/bin/env bash
    set -euo pipefail
    config=$(mktemp "${TMPDIR:-/tmp}/recite-deny.XXXXXX")
    trap 'rm -f -- "$config"' EXIT
    sed 's/^unmaintained = "workspace"$/unmaintained = "all"/' .cargo/deny.toml > "$config"
    grep -qx 'unmaintained = "all"' "$config" || { echo 'The dependency policy changed; update this review.' >&2; exit 1; }
    status=0
    for manifest in Cargo.toml editors/zed/Cargo.toml apps/writer/Cargo.toml; do
        echo "Reviewing $manifest"
        cargo deny --locked --manifest-path "$manifest" --all-features --config "$config" check advisories || status=1
    done
    exit "$status"

# Produce local CLI/LSP artifacts using cargo-dist's archive and checksum machinery.
build target:
    mkdir -p target/distrib
    mise -E release exec -- dist build --artifacts=local --target "$1" --output-format=json > "target/$1-dist-manifest.json"
    mv "target/$1-dist-manifest.json" target/distrib/

# Check the default-feature public Rust API against an explicitly accepted revision.
# "initial" records the absence of a published baseline; it makes no compatibility claim.
api-check baseline:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "API candidate: $(git rev-parse HEAD)"
    if [[ "$1" == initial ]]; then
        test -z "$(git tag --list 'v[0-9]*')" || { echo 'A release baseline exists; supply its revision.' >&2; exit 1; }
        echo 'Initial release: no published API baseline; compatibility is unmeasured.'
        exit 0
    fi
    baseline=$(git rev-parse --verify --end-of-options "$1^{commit}")
    test "$baseline" != "$(git rev-parse HEAD)" || { echo 'API baseline must precede the candidate.' >&2; exit 1; }
    git merge-base --is-ancestor "$baseline" HEAD
    echo "API baseline: $baseline; candidate: $(git rev-parse HEAD); default features; additive-only budget"
    # Explicit crate roots avoid duplicate package names in editor-parity fixtures.
    archive=$(mktemp -d "${TMPDIR:-/tmp}/recite-api-baseline.XXXXXX")
    trap 'rm -rf -- "$archive"' EXIT
    git archive "$baseline" | tar -x -C "$archive"
    for crate in core parser compiler runtime build config import adapter ui; do
        mise -E release exec -- cargo semver-checks --manifest-path "crates/recite-$crate/Cargo.toml" --baseline-root "$archive/crates/recite-$crate" --release-type minor --default-features
    done
