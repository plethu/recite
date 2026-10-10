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
# Release preparation, distribution planning and candidate verification.
mod release 'release.just'

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
    just _clippy-rust

[private]
_clippy-rust:
    cargo clippy --workspace --locked --all-targets --all-features -- -D warnings -D clippy::excessive_nesting
    # Assertion-heavy scenarios distort cognitive complexity; nesting covers all targets.
    cargo clippy --workspace --locked --all-features --lib --bins -- -D warnings -D clippy::cognitive_complexity -D clippy::excessive_nesting

# Core semantics and Rust APIs; host and Writer checks run independently in CI.
core-check:
    cargo fmt --all -- --check
    just test
    just test-doc
    just _clippy-rust
    RUSTDOCFLAGS=-Dwarnings cargo doc --locked --workspace --all-features --no-deps

# Real host conformance and clean package consumers remain correctness checks.
host-check:
    scripts/generate-ffi-header.sh
    scripts/check-ffi-header.sh
    just engines unity
    just engines godot
    cargo fetch --locked
    just engines bevy
    just engines workflow

# Native editor and grammar contracts, separate from the TypeScript client lane.
editor-native-check:
    scripts/check-test-organization.sh
    scripts/check-editor-parity.sh
    scripts/check-tree-sitter.sh
    scripts/check-neovim.sh
    scripts/check-zed.sh
    scripts/check-lint-suppressions.sh

test *args:
    cargo nextest run --workspace --locked "$@"

test-doc *args:
    cargo test --workspace --locked --doc "$@"

# Enforce production line coverage per owner; tests, examples and benches are not production.
coverage:
    cargo llvm-cov clean --workspace
    cargo llvm-cov nextest --workspace --locked --all-features --test-threads 4 --no-fail-fast --no-report
    mkdir -p target/coverage
    cargo llvm-cov report --json --summary-only --ignore-filename-regex '/(tests|benches|examples)/' --output-path target/coverage/workspace.json
    for owner in recite-core:90 recite-parser:95 recite-compiler:90 recite-runtime:90 recite-lsp:90 recite-cli:80; do package="${owner%:*}"; floor="${owner#*:}"; cargo llvm-cov report --package "$package" --json --summary-only --ignore-filename-regex '/(tests|benches|examples)/' --fail-under-lines "$floor" --output-path "target/coverage/$package.json"; done

# All viable critical mutations must be caught; retain the baseline and downstream consumers.
mutants *args:
    #!/usr/bin/env bash
    set -euo pipefail
    for arg in "$@"; do
        case "$arg" in
            --shard | --jobs | --build-timeout) ;;
            *) [[ "$arg" =~ ^[0-9]+(/[0-9]+)?$ ]] || { echo 'just mutants accepts only --shard, --jobs and --build-timeout; use cargo mutants for scope experiments.' >&2; exit 2; } ;;
        esac
    done
    critical="$(cargo metadata --locked --no-deps --format-version 1 | jq -ce '.metadata.recite.critical_mutation_files | if type == "array" and length > 0 and all(.[]; type == "string") then . else error("Missing or invalid critical mutation file list") end')"
    file_args=()
    while IFS= read -r file; do file_args+=(--file "$file"); done < <(jq -r '.[]' <<< "$critical")
    mkdir -p target/mutation
    cargo mutants --workspace --list --json "${file_args[@]}" > target/mutation/critical-mutants.json
    jq -e --argjson expected "$critical" '($expected - ([.[].file] | unique)) as $missing | if ($missing | length) == 0 then true else error("No mutation candidates for: " + ($missing | join(", "))) end' target/mutation/critical-mutants.json > /dev/null
    # cargo-mutants' baseline omits configured downstream consumers; check them first.
    cargo nextest run --locked -p recite-core -p recite-compiler -p recite-lsp -p recite-runtime --cargo-profile mutants --test-threads 4
    cargo mutants --workspace --output target/mutation "${file_args[@]}" "$@"

supply-chain:
    scripts/check-dependencies.sh

unused-deps:
    cargo machete crates tools editors/zed apps/writer/crates

spelling:
    typos

# Check source size, structural rules and lint-suppression regressions.
maintainability:
    mise -E maintainability exec -- scripts/check-source-policy.sh
    mise -E maintainability exec -- ast-grep test --config tools/ast-grep/sgconfig.yml --skip-snapshot-tests
    mise -E maintainability exec -- ast-grep scan --config tools/ast-grep/sgconfig.yml

check:
    mise -E maintainability exec -- just _verify

[private]
_verify:
    just _check-quality
    scripts/check-git-policy.sh
    bash tests/git-policy/check-integration.sh
    bash tests/trusted-policy/check.sh
    bash tests/editor-parity/check.sh
    scripts/check-vscode.sh
    scripts/check-helix.sh
    tests/editor-hosts/helix/check.sh
    scripts/check-project-gates.sh
    scripts/check-docs.sh
    just perf smoke
    just coverage
    just writer coverage
    just mutants --jobs 2 --build-timeout 600

# Shared local and CI quality lane; commands have one owner.
[private]
_check-quality:
    scripts/install-js-dependencies.sh
    just quality setup
    just fmt-check
    just quality lint
    just spelling
    just unused-deps
    just supply-chain
    actionlint -shellcheck= -pyflakes=
    just perf setup
    just perf check
    tests/maintainability/check.sh
    tests/maintainability/format-replay.sh
    tests/ast-grep/check.sh
    just maintainability
    tests/lint-suppressions/check.sh

verify: check
