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
# Formatting and linting across maintained source languages.
mod quality 'quality.just'

default:
    @just --list

# Install JavaScript workspace dependencies from the committed lockfile.
setup:
    just quality setup
    scripts/install-js-dependencies.sh
    just perf setup

fmt:
    cargo fmt --all
    just editor zed fmt
    just quality fmt

fmt-check:
    cargo fmt --all -- --check
    just editor zed fmt-check
    just quality fmt-check

lint:
    just quality lint
    just clippy

clippy:
    just editor zed clippy
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings
    # Assertion-heavy scenarios distort this metric; enforce it on production targets.
    cargo clippy --workspace --locked --all-features --lib --bins -- -D warnings -D clippy::cognitive_complexity

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

# Check source size, structural rules and lint-suppression regressions.
maintainability:
    scripts/check-maintainability.sh
    mise -E maintainability exec -- ast-grep test --config tools/ast-grep/sgconfig.yml --skip-snapshot-tests
    mise -E maintainability exec -- ast-grep scan --config tools/ast-grep/sgconfig.yml
    mise -E maintainability exec -- scripts/check-lint-suppressions.sh

check:
    mise -E maintainability exec -- just _verify

[private]
_verify:
    scripts/install-js-dependencies.sh
    just quality setup
    just fmt-check
    just quality lint
    just spelling
    just unused-deps
    just supply-chain
    actionlint -shellcheck= -pyflakes=
    scripts/check-git-policy.sh
    bash tests/git-policy/check-integration.sh
    just perf setup
    just perf check
    tests/maintainability/check.sh
    tests/maintainability/format-replay.sh
    tests/ast-grep/check.sh
    just maintainability
    tests/lint-suppressions/check.sh
    bash tests/trusted-policy/check.sh
    bash tests/editor-parity/check.sh
    scripts/check-vscode.sh
    scripts/check-helix.sh
    tests/editor-hosts/helix/check.sh
    scripts/check-project-gates.sh
    scripts/check-docs.sh
    just perf smoke

verify: check
