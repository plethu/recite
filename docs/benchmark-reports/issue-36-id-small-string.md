# Issue 36: ID Small-String Evaluation

Date: 2026-06-01

## Decision

Recommend a follow-up implementation issue to switch the shared `recite-core`
ID macro from `String` to `compact_str::CompactString`, after review of the
dependency as a public library dependency. The measured fixtures show meaningful
retained heap-payload reduction for ID-heavy AST and compiled-dialogue data, no
increase in ID wrapper or compiled row sizes on this target, and no clear timing
regression in the one-sample release measurements.

This branch keeps the change reversible through the `small-ids` feature:

- default build: `String`-backed IDs;
- experimental build: `compact_str`-backed IDs through
  `recite-benchmarks/small-ids`.

The optional `compact_str` dependency is used only for the experimental variant
in this branch. It was already present in `Cargo.lock` through `ratatui`, is MIT
licensed, and keeps `LineId`, `ChoiceId`, `BlockId`, `EffectId`, `LocaleId`, and
`SpeakerId` at 24 bytes on this target. If the switch is rejected, remove the
feature and dependency wiring.

## Scope Checked

Public ID APIs remain unchanged:

- `new(value: impl Into<String>)`;
- `as_str() -> &str`;
- `Display`;
- `TryFrom<&str>` and `TryFrom<String>`;
- messagepack/session serialization surfaces continue to convert through
  strings.

## Commands

```bash
cargo test -p recite-core -p recite-benchmarks
cargo test -p recite-core -p recite-benchmarks --features small-ids

cargo run -p recite-benchmarks --release --bin id_memory_report -- \
  --variant string \
  --scales tiny,small,medium,large,epic \
  --repeat 1

cargo run -p recite-benchmarks --release --features small-ids --bin id_memory_report -- \
  --variant compact_str \
  --scales tiny,small,medium,large,epic \
  --repeat 1
```

The report metrics count retained ID newtypes in source ASTs, compiled dialogue
assets including lookup tables, and runtime fixture IDs. Heap payload estimates
count ID string bytes only, not allocator metadata, so the real allocation
pressure of `String` can be higher.

## Structural Results

On this target, `String`, `CompactString`, and every ID newtype are 24 bytes.
The measured compiled row sizes are unchanged by the feature:

| Type | Size |
| --- | ---: |
| `LineId` and other ID wrappers | 24 B |
| `CompiledBlock` | 56 B |
| `CompiledLine` | 72 B |
| `CompiledChoice` | 144 B |
| `CompiledEffect` | 80 B |
| `DialogueSession` | 456 B |

`compact_str` inlines strings up to 24 bytes on this target. All generated
source IDs, line IDs, choice IDs, block IDs, speakers, locales, and runtime
fixture choice IDs fit inline. Generated compiled effect IDs remain heap-backed
because they are 32-34 bytes.

| Scale | Source IDs | Source heap `String` -> `compact_str` | Compiled IDs | Compiled heap `String` -> `compact_str` | Runtime fixture heap `String` -> `compact_str` |
| --- | ---: | ---: | ---: | ---: | ---: |
| tiny | 262 | 3,172 B -> 0 B | 273 | 3,896 B -> 156 B | 85 B -> 0 B |
| small | 2,647 | 32,017 B -> 0 B | 2,644 | 37,854 B -> 1,174 B | 805 B -> 0 B |
| medium | 26,497 | 320,467 B -> 0 B | 26,352 | 377,630 B -> 11,550 B | 8,005 B -> 0 B |
| large | 132,497 | 1,602,467 B -> 0 B | 131,724 | 1,888,134 B -> 58,054 B | 40,005 B -> 0 B |
| epic | 224,997 | 2,724,967 B -> 0 B | 223,438 | 3,216,056 B -> 115,976 B | 80,005 B -> 0 B |

For the epic fixture, the compiled asset ID heap-payload estimate drops by about
3.1 MB, before allocator overhead. The source AST estimate drops by about 2.7 MB.

## Timing Results

Single-sample release timings are suitable only as a smoke comparison. They do
not replace Criterion trend runs.

| Scale | Variant | Parse ms | Lower ms | Compile ms | Full traversal ms |
| --- | --- | ---: | ---: | ---: | ---: |
| tiny | `String` | 0.075 | 0.240 | 1.762 | 0.032 |
| tiny | `compact_str` | 0.074 | 0.223 | 1.645 | 0.031 |
| small | `String` | 1.245 | 2.079 | 18.772 | 0.203 |
| small | `compact_str` | 0.661 | 2.140 | 17.948 | 0.177 |
| medium | `String` | 6.201 | 26.510 | 231.360 | 2.117 |
| medium | `compact_str` | 6.427 | 29.180 | 232.784 | 2.022 |
| large | `String` | 33.688 | 131.084 | 1,191.382 | 13.272 |
| large | `compact_str` | 32.805 | 126.359 | 1,219.225 | 12.819 |
| epic | `String` | 58.715 | 227.119 | 2,101.700 | 30.809 |
| epic | `compact_str` | 62.171 | 230.562 | 2,102.817 | 28.725 |

The largest compile delta was `large`, where `compact_str` was about 2.3% slower
in a one-sample run. The `epic` compile delta was about 0.05% slower. Full
traversal was slightly faster in the larger fixtures. Treat these as noise until
Criterion baselines are collected on a stable runner.

## Follow-Up Issue Shape

Title: Switch shared ID newtypes to compact string storage

Body:

- Use `compact_str::CompactString` in the shared `recite-core` ID macro.
- Keep public ID APIs and wire/session serialization unchanged.
- Re-run Criterion compiler/runtime benches on a stable runner with
  `RECITE_BENCH_SCALES=tiny,small,medium,large,epic`.
- Verify `cargo test`, `cargo clippy --all-targets --all-features -- -D warnings`,
  and benchmark smoke.
- Re-check dependency policy: MIT license, already present in lockfile through
  `ratatui`, direct dependency is justified by reduced ID allocation pressure.

Acceptance:

- ID wrapper sizes stay at 24 bytes on the supported target profile.
- No public API or serialized format changes.
- No sustained compiler/runtime timing regression above Recite's review
  thresholds.
- Reported ID heap-payload savings remain in the same order of magnitude on the
  generated scale ladder.
