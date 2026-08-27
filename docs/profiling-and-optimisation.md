# Recite Profiling and Optimisation Playbook

This playbook is for maintainers investigating Recite performance work. It
turns a suspicious benchmark result, trace, or authoring-loop delay into a
repeatable profile and a concrete optimisation hypothesis.

The production requirements live in
[`docs/recite-production-spec.md`](recite-production-spec.md) section 19.
Numbers in that section are aspirational until a release baseline exists. Before
that baseline, local measurements are evidence for review and investigation, not
hard pass/fail gates.

## Investigation Workflow

Use the same sequence for compiler, runtime, LSP, watch, and memory work:

1. Confirm the symptom with the smallest relevant benchmark or command.
2. Re-run enough times to decide whether the signal is stable or local noise.
3. Capture a CPU, allocation, or memory profile for the suspected path.
4. Write one optimisation hypothesis that names the hot path, expected cause,
   and expected metric movement.
5. Change code only after the hypothesis is specific enough to review.
6. Re-run the same benchmark or profile command before widening the claim.
7. Record the result, caveats, and follow-up issue links in the PR or report.

Do not tune against a single laptop timing. Local runs are useful for finding a
cause. Trend claims and release comparisons should come from one documented
Linux runner or the release baseline profile tracked by [#109 Perf: establish
release benchmark baseline profile](https://github.com/plethu/recite/issues/109).

## Measurement Profiles

Use two profiles deliberately:

- Local Linux diagnostic profile: the normal maintainer workflow for finding a
  likely cause. Record CPU model, kernel, Rust toolchain, command, fixture
  selector, git commit, and whether the machine was on AC power and otherwise
  idle.
- Stable trend profile: the only source for release notes, blocking regression
claims, and cross-PR trend comparisons. Until [#109 Perf: establish release
benchmark baseline profile](https://github.com/plethu/recite/issues/109) defines
it, treat trend
  numbers as provisional.

Criterion is the first timing surface. Prefer the existing benchmark targets
before opening lower-level profilers:

```bash
cargo bench -p recite-benchmarks --no-run
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp
```

Use `RECITE_BENCH_SCALES=tiny,small` as the quick smoke path when validating a
compiler benchmark change locally. Move to medium or large only after the small
run identifies a candidate path or the issue is explicitly about scale shape:

```bash
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=large cargo bench -p recite-benchmarks --bench compiler
```

For quick build/execution smoke, use:

```bash
scripts/benchmark-smoke.sh
```

The smoke proves the tiny compiler and runtime Criterion targets build and run.
It does not compare timing.

## Interpreting Criterion Output

Treat Criterion output as a statistical signal, not a verdict:

- A single outlier is not a regression. Re-run before investigating unless the
  change is huge or repeatable.
- Local laptop variance can come from CPU scaling, thermal throttling, editor
  indexing, background builds, and battery mode.
- Compare the same fixture selector, benchmark group, Rust toolchain, and build
  profile.
- Use tiny fixtures to localise failures quickly; use medium, large, or
  realistic fixtures to expose algorithmic shape and memory pressure.
- Regression thresholds from the benchmark reference are review triggers until
  [#109 Perf: establish release benchmark baseline profile](https://github.com/plethu/recite/issues/109)
  establishes the release baseline profile.

If a run is noisy, lower the claim instead of overfitting the result. For
example, say "runtime/condition_dispatch on medium should be profiled" rather
than "condition dispatch regressed by 7%" when the confidence interval overlaps
the prior run.

## CPU Profiling

Use Linux `perf` as the primary low-level profiler. It is external tooling: do
not add it, flamegraph scripts, or GPL-licensed helper code as workspace
dependencies.

Capture an authoritative local CPU profile by running exactly one benchmark
group and one scale:

```bash
RECITE_BENCH_SCALES=medium \
  perf record --call-graph dwarf -- \
  cargo bench -p recite-benchmarks --bench runtime -- runtime/full_traversal

perf report
```

For flamegraph output, install flamegraph tooling outside the repo and keep the
generated SVG out of source control unless a report explicitly needs it:

```bash
RECITE_BENCH_SCALES=medium \
  cargo flamegraph --bench runtime -- runtime/full_traversal
```

Use the same pattern for compiler and LSP groups:

```bash
RECITE_BENCH_SCALES=medium \
  perf record --call-graph dwarf -- \
  cargo bench -p recite-benchmarks --bench compiler -- compiler/validate_with_schema

RECITE_BENCH_SCALES=medium \
  perf record --call-graph dwarf -- \
  cargo bench -p recite-benchmarks --bench lsp -- lsp/diagnostics_refresh
```

When `perf` cannot be used, keep the fallback explicit in the report. Criterion
with a narrow group filter is acceptable for triage; it is not a substitute for
a CPU profile when the issue is algorithmic.

## Memory and Allocation Profiling

Use memory tools when the symptom is allocation pressure, peak memory, or clone
growth rather than elapsed time. Keep these tools optional and external:

- `heaptrack` for allocation flamegraphs and retained allocations;
- Valgrind Massif for peak heap shape when overhead is acceptable;
- allocator counters or custom measurement binaries for focused reports;
- existing Recite benchmark helpers for size-oriented reports.

Start with existing commands:

```bash
cargo run -p recite-benchmarks --release --bin memory_profile_report -- \
  --fixtures tiny,small,medium,large,epic,realistic:v1-pack \
  --format markdown \
  --output docs/benchmark-reports/issue-105-memory-profiles-known-limits.md
cargo run -p recite-benchmarks --release --bin id_memory_report -- --scales tiny,small
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks --bench lsp -- lsp/initial_index
```

Then profile the narrow path:

```bash
RECITE_BENCH_SCALES=medium \
  heaptrack cargo bench -p recite-benchmarks --bench runtime -- runtime/full_traversal

RECITE_BENCH_SCALES=medium \
  valgrind --tool=massif \
  cargo bench -p recite-benchmarks --bench compiler -- compiler/compile_with_schema
```

[#70 Perf: report memory profiles and known scale limits](https://github.com/plethu/recite/issues/70)
owns release-facing memory profiles and known scale limits. Keep
`docs/benchmark-reports/issue-105-memory-profiles-known-limits.md` generated
from `memory_profile_report`, and do not turn one local heap profile into a
release limit.

## Surface-Specific Commands

Compiler investigations usually start with validation, targeted compiler phase
checks, serialization size, and full compilation:

```bash
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/validate
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/validate_with_schema
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/block_reference_resolution
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/id_uniqueness
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/markup_validation
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/pot_extraction_pressure
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/compiled_asset_serialization
RECITE_BENCH_SCALES=tiny,small cargo bench -p recite-benchmarks --bench compiler -- compiler/compile_with_schema
```

Use medium or large for those targeted compiler checks when the smoke run points
at ID uniqueness, block reference resolution, markup validation, POT extraction,
or asset serialization. Do not add hard pass/fail thresholds from local timing
runs; record the fixture selector, command, commit, and machine profile instead.

Runtime investigations usually start with traversal, choice selection,
condition dispatch, effect emission, localisation lookup, and session
serialization:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- runtime/full_traversal
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- runtime/choose_first
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- runtime/condition_dispatch
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- runtime/localised_next
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- runtime/session_encode
```

LSP investigations usually start with indexing, edit refresh, diagnostics,
completion, definition, and rename:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/initial_index
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/change_refresh
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/diagnostics_refresh
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/completion
```

Watch/build refresh investigations are owned by [#108 CLI: add watch rebuild
latency stress checks](https://github.com/plethu/recite/issues/108). The dedicated
stress command is available as `mise run watch-stress`; measure the whole
command externally as supplementary evidence and profile the compiler path that
dominates the refresh:

```bash
mise run watch-stress
/usr/bin/time -v cargo run -p recite-cli -- watch <project-root>
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks --bench compiler -- compiler/compile_with_schema
```

Use trace metrics for real project traversal triage without writing a new Rust
benchmark:

```bash
cargo run -p recite-cli -- trace <asset> --block <block> --fixture <fixture> --metrics
```

## Hot Paths

Recite should aim to be best in class on these paths:

- runtime traversal after asset load;
- choice lookup by stable ID;
- condition dispatch and availability reporting;
- compiler validation and schema validation;
- block reference resolution and ID uniqueness checks;
- LSP diagnostics, completion, definition, and rename;
- `recite watch` rebuild latency for source, schema, and project edits.

Optimisations in these paths must preserve deterministic traversal, stable IDs,
structured diagnostics, and typed effect requests. Runtime code still must not
perform game-side effects.

## `recite bench` Mapping

[#87 CLI: add recite bench command](https://github.com/plethu/recite/issues/87)
added the user-facing `recite bench` command. Maintainers should
still use `cargo bench`, helper scripts, and low-level profilers for focused
investigation; `recite bench` is the stable project and fixture report surface.

The command maps the common report flows to:

```bash
recite bench <fixture-or-project> --group runtime --scale medium --format json --output target/recite-benchmarks/runtime.json
recite bench <fixture-or-project> --group compiler --scale medium --format markdown --baseline baselines/release.json
recite bench <fixture-or-project> --group lsp --scale tiny
```

`recite bench` currently supports `compiler`, `runtime`, and `lsp` groups.
Watch/build stress is a separate integration check through
`mise run watch-stress`, not a fourth benchmark group.

The PR and main-branch workflow keeps the tiny benchmark smoke check fast and
non-comparative. Issue [#109 Perf: establish release benchmark baseline
profile](https://github.com/plethu/recite/issues/109) owns the fuller
release/scheduled benchmark suite and named regression profile; issue [#77
Release: define v1 release candidate checklist and gate
matrix](https://github.com/plethu/recite/issues/77) owns the evidence ledger and
release-gate decision that consume it.

The command should keep JSON output suitable for CI comparison, Markdown output
for release notes, group filtering, scale selection, and baseline comparison.
It should not make profiling tools linked project dependencies.

## Issue Links

- [#70](https://github.com/plethu/recite/issues/70) owns memory profiles and
  release known-limit reporting.
- [#87](https://github.com/plethu/recite/issues/87) closed the user-facing
  `recite bench` report surface.
- [#106](https://github.com/plethu/recite/issues/106) owns targeted compiler
  phase benchmark expansion.
- [#107](https://github.com/plethu/recite/issues/107) owns runtime allocation
  and clone-pressure measurement.
- [#108](https://github.com/plethu/recite/issues/108) owns watch rebuild latency
  stress checks through `mise run watch-stress`.
- [#109](https://github.com/plethu/recite/issues/109) owns the release benchmark
  baseline profile, the fuller release/scheduled suite, and any blocking trend
  claims.
- [#77](https://github.com/plethu/recite/issues/77) owns the release evidence
  ledger and gate decision for the benchmark results.
