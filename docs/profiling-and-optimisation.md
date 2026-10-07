# Recite Profiling and Optimisation Playbook

This playbook is for maintainers investigating Recite performance work. It
turns a suspicious benchmark result, trace, or authoring-loop delay into a
repeatable profile and a concrete optimisation hypothesis.

The production requirements live in
[`docs/recite-production-spec.md`](recite-production-spec.md) section 19.
Release targets in that section are aspirational until a release baseline
exists. Local measurements are evidence for investigation, not new CI budgets.
The paired LSP regression gate already has an explicit baseline and rerun policy
in section 19.8; keep that protection distinct from discovery profiling.

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

### Broad optimisation passes

A request to optimise a subsystem or investigate remaining performance wins
requires bounded discovery, even when latency and CI checks are green. Start
by identifying the ownership that could be simplified or delegated to maintained
ecosystem packages. Count replacement glue and tests alongside deleted code;
for LSP work, consult the existing
[dependency decisions](design/lsp-cancellation/dependency-decisions.md).
Then collect actual Criterion measurements and both CPU and allocation profiles
for representative paths. Do not wait for a reported memory problem.
A narrowly scoped fix needs only the measurements relevant to its hypothesis;
it does not require repeating a subsystem audit.

Choose an authored realistic fixture and a larger stress fixture. Separate
startup from repeated warm operations; include fallback/recovery and large
output paths when the subsystem has them. Read benchmark setup and teardown
before interpreting results. For an LSP pass, include the running server:
`LspWorkspace` benchmarks bypass protocol handling and coordinator scheduling.

Record the following evidence, or mark it unmeasured with a concrete reason:

| Metric | Evidence |
| --- | --- |
| Wall latency | Actual benchmark estimates or repeated process timings |
| CPU | Sampled stacks identifying work, with capture/loss limitations |
| Allocation churn | Allocated bytes and allocation count per operation, with call sites |
| Live and peak heap | Heap at named lifecycle checkpoints and its high-water mark |
| Process memory | RSS separately from heap and estimated model sizes |

Criterion measures timing; `--test` smoke only proves execution. RSS and model
size estimates do not identify allocation churn. Instrumented runs identify
causes; use uninstrumented binaries for timing comparisons. Build before
profiling and identify the measured phase so compilation, fixture construction,
startup and teardown are not mistaken for steady-state work.

Inspect the attributed paths for simpler fixes first: known output sizes,
collection growth, unnecessary clones, repeated conversions and repeated
analysis. Reserve capacity only where a useful size is available; do not add
caches, custom representations or dependencies without evidence that the
benefit justifies their maintenance cost. Performance and maintainability are
joint acceptance criteria: a roughly 5% slowdown can be acceptable for a
substantial net simplification, subject to relevant workload evidence and
unchanged correctness. The
[final LSP resource investigation](design/lsp-cancellation/final-resource-profiling.md)
shows allocation profiles finding avoidable vector growth after latency work.

Before closing a broad pass, refresh profiles on the final implementation where
substantial changes could have moved the hotspots. Preserve commands, build and
fixture identities, phase boundaries, raw evidence, semantic parity and
alternating control/candidate comparisons for retained changes. Name the
remaining dominant costs, rejected experiments and concrete reevaluation
triggers. Report missing profiler access or incomplete coverage as limitations;
passing smoke or latency gates alone does not complete this investigation.
Inspect the final code and callers for readable ownership and unnecessary
indirection. Remove superseded implementations and retire concluded diagnostic
tools or CI controls; retain their evidence and fixed-revision reproduction
instead of maintaining every experiment indefinitely.

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
  claims beyond the existing paired LSP gate, and cross-PR trend comparisons.
  Until [#109 Perf: establish release benchmark baseline profile](https://github.com/plethu/recite/issues/109)
  defines it, treat trend numbers as provisional.

Criterion is the first timing surface. Prefer the existing benchmark targets
before opening lower-level profilers. Maintainer commands are exposed through
`just perf`; its scoped `setup` installs the locked external-process harness:

```bash
just perf setup
just perf bench lsp large,realistic:v1-pack 'lsp/change_refresh'
just perf compare BASE_COMMIT
just perf lsp --help
```

Use raw Cargo commands when a profiler needs an executable or a custom build:

```bash
cargo bench -p recite-benchmarks --no-run
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview
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

The smoke proves the tiny compiler, runtime, preview, and LSP Criterion targets
build and run. It does not compare timing or attribute CPU and allocations.

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

Build first, then use the executable path printed by Cargo (including its hash)
to profile one group and one scale. Criterion's `--profile-time` repeats the
workload without statistical analysis; these samples do not replace a timing
baseline. For example:

```bash
cargo bench --locked -p recite-benchmarks --bench runtime --no-run
bench_bin=/absolute/path/from/cargo/output/runtime-HASH
RECITE_BENCH_SCALES=medium \
  perf record --call-graph dwarf -- \
  "$bench_bin" --bench 'runtime/full_traversal/medium' --profile-time 15

perf report
```

For flamegraph output, install flamegraph tooling outside the repo and keep the
generated SVG out of source control unless a report explicitly needs it:

```bash
RECITE_BENCH_SCALES=medium \
  cargo flamegraph --bench runtime -- runtime/full_traversal
```

Use the same build-then-profile pattern for compiler and LSP groups. Select the
matching executable and filter, such as `compiler/validate_with_schema/medium`
or `lsp/diagnostics_refresh/medium`. Benchmark fixture setup and batched
preparation still execute in profiling mode; inspect stacks or use a focused
process probe when the question requires isolating a warm operation. Preserve
the profiler build flags, unresolved frames and lost-sample counts in evidence.

When `perf` cannot be used, keep the fallback explicit in the report. Criterion
with a narrow group filter is acceptable for triage; it is not a substitute for
a CPU profile when the issue is algorithmic.

## Memory and Allocation Profiling

Use memory tools for allocation pressure, peak memory and clone growth, and as
part of the bounded discovery required for broad optimisation passes. Keep
instrumentation out of production builds:

- `heaptrack` for allocation flamegraphs and retained allocations;
- Valgrind Massif for peak heap shape when overhead is acceptable;
- DHAT in an isolated diagnostic build for allocation sites and lifecycle counters;
- allocator counters or custom measurement binaries for focused reports;
- existing Recite benchmark helpers for size-oriented reports.

Start with existing commands:

```bash
cargo run -p recite-benchmarks --release --bin memory_profile_report -- \
  --fixtures tiny,small,medium,large,epic,realistic:v1-pack \
  --format markdown \
  --output /tmp/recite-memory-profiles.md
cargo run -p recite-benchmarks --release --bin id_memory_report -- --scales tiny,small
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks --bench lsp -- lsp/initial_index
```

Size reports are estimates of selected structures, not heap allocation traces
or process RSS. Then profile the narrow path using a prebuilt executable:

```bash
cargo bench --locked -p recite-benchmarks --bench runtime --no-run
bench_bin=/absolute/path/from/cargo/output/runtime-HASH
RECITE_BENCH_SCALES=medium \
  heaptrack "$bench_bin" --bench 'runtime/full_traversal/medium' --profile-time 15

cargo bench --locked -p recite-benchmarks --bench compiler --no-run
bench_bin=/absolute/path/from/cargo/output/compiler-HASH
RECITE_BENCH_SCALES=medium \
  valgrind --tool=massif \
  "$bench_bin" --bench 'compiler/compile_with_schema/medium' --profile-time 15
```

[#70 Perf: report memory profiles and known scale limits](https://github.com/plethu/recite/issues/70)
owns release-facing memory profiles and known scale limits. Generate the report with `memory_profile_report` for each evaluation, and
do not turn one local heap profile into a release limit.

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

Preview investigations usually start with full traversal, snapshot encoding,
restore, and retained trace shape:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview -- preview/full_traversal
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview -- preview/snapshot_encode
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview -- preview/restore
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview -- preview/retained_trace_shape
```

Traversal and step report throughput in events, while snapshot encoding and
restore report bytes processed. Retained trace shape intentionally remains a
per-report timing: its structured counters do not represent one meaningful
throughput unit. `preview/full_traversal` uses a black-box event sink and count
only; `preview/evidence_report` is the exhaustive event/state BLAKE3 evidence
path and is intentionally measured separately.

LSP investigations usually start with indexing, edit refresh, diagnostics,
completion, definition, and rename:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/initial_index
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/change_refresh
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/diagnostics_refresh
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- lsp/completion
```

Watch/build refresh investigations now use the generated-fixture stress command
implemented by closed issue [#108 CLI: add watch rebuild latency stress
checks](https://github.com/plethu/recite/issues/108). Run the dedicated stress
check through the repository task; any new investigations follow the current
benchmark ownership below:

```bash
mise run watch-stress
```

For supplementary whole-command timing, or when an external profiler needs the
real watch process, use:

```bash
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
- [#108](https://github.com/plethu/recite/issues/108) implemented the watch
  rebuild latency stress checks through `mise run watch-stress` and is closed.
- [#109](https://github.com/plethu/recite/issues/109) owns the release benchmark
  baseline profile, the fuller release/scheduled suite, and any blocking trend
  claims.
- [#77](https://github.com/plethu/recite/issues/77) owns the release evidence
  ledger and gate decision for the benchmark results.
