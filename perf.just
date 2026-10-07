set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list := true

# Install the scoped runner and sync the locked Python 3.12 environment.
setup:
    mise -E lsp install uv
    mise -E lsp exec -- uv sync --locked

# Run the LSP tooling's formatter, linter and CI contract tests.
check:
    mise -E lsp exec -- uv run --locked ruff format --check
    mise -E lsp exec -- uv run --locked ruff check
    mise -E lsp exec -- uv run --locked python -m unittest discover -s tests/ci -p 'test_*.py'

# Deliberately format maintained LSP Python tools and their tests.
fmt:
    mise -E lsp exec -- uv run --locked ruff format

# Measure a Criterion target on selected fixture scales; trailing arguments go to Criterion.
bench target scales="tiny,small" *args:
    case "$1" in compiler|runtime|preview|lsp) ;; *) echo 'Expected compiler, runtime, preview, or lsp' >&2; exit 2 ;; esac
    RECITE_BENCH_SCALES="$2" cargo bench --locked -p recite-benchmarks --bench "$1" -- "${@:3}"

# Check benchmark execution without collecting timing baselines.
smoke:
    scripts/benchmark-smoke.sh

# Build the process probes' server and fixture generator before measurement.
build:
    cargo build --locked --release -p recite-lsp -p recite-fixturegen

# Build and compare release LSP revisions, including bounded session probes.
compare base:
    scripts/check-lsp-performance.sh "$1"

# Run the same fixed-set/churn repetitions as CI (build first with perf build).
sessions *args:
    mise -E lsp exec -- uv run --locked python scripts/lsp.py matrix "$@"

# Run one external LSP probe or report check; use --help to list commands.
lsp *args:
    mise -E lsp exec -- uv run --locked python scripts/lsp.py "$@"
