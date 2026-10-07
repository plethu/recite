set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Install the scoped runner and sync the locked Python 3.12 environment.
setup:
    mise -E lsp exec -- uv sync --locked --python "$(mise which python)" --no-python-downloads

# Run LSP harness and CI contract tests; quality owns formatting and linting.
check:
    mise -E lsp exec -- uv run --locked python -m unittest discover -s tests/ci -p 'test_*.py'

# Inspect the same changed-path selection used by GitHub Actions.
scope base="origin/main" head="HEAD":
    mise -E lsp exec -- uv run --locked python scripts/ci-scope.py --base "$1" --head "$2"

# Measure a Criterion target on selected fixture scales; trailing arguments go to Criterion.
bench target scales="tiny,small" *args:
    case "$1" in compiler|runtime|preview|lsp|sessions) ;; *) echo 'Expected compiler, runtime, preview, lsp, or sessions' >&2; exit 2 ;; esac
    RECITE_BENCH_SCALES="$2" cargo bench --locked -p recite-benchmarks --bench "$1" -- "${@:3}"

# Check benchmark execution without collecting timing baselines.
smoke:
    scripts/benchmark-smoke.sh

# Report warm adapter choice and preview replay allocations at bounded history sizes.
allocations:
    cargo run --locked --release -p recite-benchmarks --example session_allocations

# Measure the real C ABI: registry lookup, callbacks, rollback and host encoding.
ffi *args:
    cargo bench --locked -p recite-ffi --bench sessions -- "$@"

ffi-allocations:
    cargo run --locked --release -p recite-ffi --example session_allocations

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
