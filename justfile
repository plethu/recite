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
    mise -E godot exec -- scripts/check-godot-host.sh

test-editor-host client *args:
    case "$1" in neovim|vscode|zed) scripts/check-"$1"-host.sh "${@:2}" ;; *) echo 'Expected neovim, vscode, or zed' >&2; exit 2 ;; esac
