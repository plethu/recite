set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Run the large-project CLI stress test.
scale *args:
    cargo test --locked -p recite-cli --test scale_stress -- --ignored "$@"

# Run the CLI watch-mode stress test.
watch *args:
    cargo test --locked -p recite-cli --test watch_stress -- --ignored "$@"
