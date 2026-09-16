set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

default:
    @just --list

# Install JavaScript workspace dependencies from the committed lockfile.
setup:
    scripts/install-js-dependencies.sh

fmt:
    cargo fmt --all
    cargo fmt --manifest-path editors/zed/Cargo.toml
    taplo fmt

fmt-check: fmt-zed-check
    cargo fmt --all -- --check
    taplo fmt --check
    taplo lint

fmt-zed-check:
    cargo fmt --manifest-path editors/zed/Cargo.toml -- --check

clippy: clippy-zed
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings

clippy-zed:
    cargo clippy --locked --manifest-path editors/zed/Cargo.toml --all-targets -- -D warnings

test *args:
    cargo nextest run --workspace --locked "$@"

test-doc *args:
    cargo test --workspace --locked --doc "$@"

test-zed *args:
    cargo nextest run --locked --manifest-path editors/zed/Cargo.toml "$@"

supply-chain:
    scripts/check-dependencies.sh

unused-deps:
    cargo machete crates editors/zed

spelling:
    typos

check:
    mise -E maintainability exec -- scripts/verify.sh

verify: check

test-stress *args:
    cargo test --locked -p recite-cli --test scale_stress -- --ignored "$@"

test-watch-stress *args:
    cargo test --locked -p recite-cli --test watch_stress -- --ignored "$@"

test-godot:
    mise -E godot install
    mise -E godot exec -- scripts/check-godot-host.sh

test-editor-host client *args:
    case "$1" in neovim|vscode|zed) scripts/check-"$1"-host.sh "${@:2}" ;; *) echo 'Expected neovim, vscode, or zed' >&2; exit 2 ;; esac

# Launch the native writer, optionally with --project PATH.
writer *args:
    cargo run --locked --manifest-path apps/writer/Cargo.toml -p recite-writer -- "$@"

# Verify the maintained native application and its source-editing model.
check-writer:
    cargo fmt --manifest-path apps/writer/Cargo.toml --all -- --check
    cargo test --locked --manifest-path apps/writer/Cargo.toml --workspace
    cargo clippy --locked --manifest-path apps/writer/Cargo.toml --workspace --all-targets --all-features -- -D warnings
    just check-writer-heap

# Fixed-corpus allocation regression checks; no wall-clock budget.
check-writer-heap:
    cargo bench --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-model --features heap-profile --bench large_project -- --passages 10000 --check-heap
    cargo bench --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-model --features heap-profile --bench large_project -- --passages 10000 --check-heap --linked

# perf (Linux CPU) or DHAT (heap), using the optimized benchmark with debug lines.
profile-writer mode="cpu" passages="10000" output="/tmp/recite-writer-profile":
    scripts/profile-writer.sh "$1" "$2" "$3"

# Generated saved-project workload; accepts --passages, --per-document and --output.
bench-writer *args:
    cargo bench --locked --manifest-path apps/writer/Cargo.toml -p recite-writer-model --features benchmarks --bench large_project -- "$@"

# Headless large-conversation mounting and navigation timings (not native FPS).
bench-writer-ui beats="1000":
    RECITE_BENCH_BEATS="$1" cargo test --locked --manifest-path apps/writer/Cargo.toml -p recite-writer --test scale -- --ignored --nocapture

# Background draft queue, durable flush and reopen timings.
bench-writer-recovery:
    cargo test --locked --manifest-path apps/writer/Cargo.toml -p recite-writer --lib recovery::tests::background_recovery_timing -- --ignored --nocapture
