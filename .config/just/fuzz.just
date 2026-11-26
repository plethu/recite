set working-directory := '../..'
set shell := ["bash", "-euo", "pipefail", "-c"]
set positional-arguments
set default-list

# Pinned nightly for libFuzzer instrumentation; stable builds never use it.
nightly := "nightly-2026-11-01"
targets := "compiled_asset session_snapshot source"

# Install the pinned nightly toolchain the fuzz targets build with.
toolchain:
    rustup toolchain install {{nightly}} --profile minimal

# Run the generated-program properties with many cases (default 4096).
properties cases="4096":
    PROPTEST_CASES={{cases}} cargo nextest run --locked --release -p recite-compiler -p recite-runtime -E 'binary(generated_programs) | binary(spec_rejections)' --no-tests fail

# Fuzz every target for `seconds` each, from a scratch copy of its seed corpus.
run seconds="60":
    for target in {{targets}}; do \
      scratch="target/fuzz-corpus/$target"; rm -rf "$scratch"; mkdir -p "$scratch"; \
      cp fuzz/corpus/"$target"/* "$scratch"/; \
      cargo +{{nightly}} fuzz run --fuzz-dir fuzz "$target" "$scratch" -- -max_total_time={{seconds}}; \
    done

# Rewrite the checked-in seed corpora from deterministic generated programs.
seed:
    cargo run --locked --manifest-path fuzz/Cargo.toml --bin seed-corpus
