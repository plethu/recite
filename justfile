set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments

# Site development and documentation.
mod web 'docs-site/justfile'
# Native writer development and performance.
mod writer 'apps/writer/justfile'
# Editor extensions and installed-host checks.
mod editor 'editors/justfile'
# Engine companion and end-to-end workflow checks.
mod engines 'engines.just'
# Expensive CLI stress checks.
mod stress 'stress.just'
# Criterion, LSP process measurements and tooling checks.
mod perf 'perf.just'

default:
    @just --list

# Install JavaScript workspace dependencies from the committed lockfile.
setup:
    scripts/install-js-dependencies.sh

fmt:
    cargo fmt --all
    just editor zed fmt
    taplo fmt

fmt-check:
    cargo fmt --all -- --check
    just editor zed fmt-check
    taplo fmt --check
    taplo lint

clippy:
    just editor zed clippy
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings

test *args:
    cargo nextest run --workspace --locked "$@"

test-doc *args:
    cargo test --workspace --locked --doc "$@"

supply-chain:
    scripts/check-dependencies.sh

unused-deps:
    cargo machete crates editors/zed

spelling:
    typos

# Inspect changed source for size, structure and lint-suppression regressions.
maintainability:
    scripts/check-maintainability.sh
    mise -E maintainability exec -- scripts/check-ast-grep.sh
    scripts/check-lint-suppressions.sh

check:
    mise -E maintainability exec -- scripts/verify.sh

verify: check
