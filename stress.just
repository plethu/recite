set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Run the large-project CLI stress test.
scale *args:
    cargo test --locked -p recite-cli --test scale_stress -- --ignored "$@"

# Run the CLI watch-mode stress test.
watch *args:
    cargo test --locked -p recite-cli --test watch_stress -- --ignored "$@"

# Fixed scheduled/release scope; missing tests fail instead of silently skipping coverage.
bounded:
    cargo nextest run --locked --release -p recite-cli --test scale_stress -E 'test(=large_scale_cli_stress)' --run-ignored only --no-tests fail
    RECITE_BENCH_SCALES=tiny,small cargo nextest run --locked --release -p recite-cli --test watch_stress --run-ignored only --no-tests fail --no-capture
