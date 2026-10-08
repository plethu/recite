# Profiling and optimisation

Performance work must justify its maintenance cost. A roughly 5% slowdown can be acceptable for a
substantial simplification; compare the workloads affected by the change and preserve semantics.

[Quality requirements](spec/quality.md) own performance policy. The paired LSP gate has a blocking
regression budget; other local measurements are investigation evidence until
[#109](https://github.com/plethu/recite/issues/109) establishes release baselines.

## Choose the measurement

Use `mise exec -- just perf …` without shell activation. Build before collecting timings and keep
other builds and tests out of the sampling window.

| Question                                       | Command                                              |
| ---------------------------------------------- | ---------------------------------------------------- |
| Compiler phase, validation or encoding         | `just perf bench compiler tiny,small FILTER`         |
| Direct runtime traversal or localisation       | `just perf bench runtime tiny,small FILTER`          |
| Preview traversal, snapshots or retained trace | `just perf bench preview tiny,small FILTER`          |
| In-process LSP analysis                        | `just perf bench lsp large,realistic:v1-pack FILTER` |
| Running LSP, protocol and scheduling           | `just perf compare BASE_COMMIT`                      |
| Growing adapter history and preview replay     | `just perf bench sessions tiny FILTER`               |
| Real C ABI traversal, callbacks and encoding   | `just perf ffi FILTER`                               |
| Warm adapter and preview allocations           | `just perf allocations`                              |
| Warm C ABI allocations                         | `just perf ffi-allocations`                          |
| Build/watch refresh under fixture pressure     | `just stress watch`                                  |
| Execution smoke, without performance claims    | `just perf smoke`                                    |

Use `just perf lsp --help` for bounded process/session probes. Its setup is `just perf setup`.
Criterion filters match group names, for example `lsp/change_refresh` or
`sessions/preview_condition_replay/deferred/2048`; `cargo bench -- --list` lists a target's cases.

The [session target](../crates/recite-benchmarks/benches/sessions.rs) owns warm-operation workload
shape and setup/teardown. The [C ABI probes](../crates/recite-ffi/benches/) own callback/encoding
parity checks and allocation boundaries; inspect those owners when interpreting a result.

Keep allocation instrumentation separate from timing binaries. Allocated bytes/counts describe
churn, not live heap or process RSS. Size reports describe selected structures, not observed
allocations; use `memory_profile_report` or `id_memory_report` only for those questions.

## Writer workloads

The [Writer recipes](../apps/writer/justfile) expose project, headless UI and recovery workloads.
The [project benchmark](../apps/writer/crates/authoring/benches/large_project.rs) owns corpus shape,
operations, reports and fixed-corpus allocation bounds. For example:

```sh
mise exec -- just writer bench --passages 100000 --linked --output target/writer-linked.json
mise exec -- just writer profile heap 10000 target/writer-heap
```

Kernel-open timing excludes filesystem discovery, indexing and painting. The
[profile script](../scripts/profile-writer.sh) records builds/environment, refuses overwrite and
runs perf or bench-only DHAT instrumentation; instrumented latency is not normal-run timing and
production allocation is unchanged. Allocation bounds run in `just writer check`; investigate
unexplained regressions before changing them. Native scale requirements belong to
[Writer acceptance](../apps/writer/acceptance.md#scale-and-performance).

## Investigation and acceptance

For a focused fix, reproduce its symptom, name the suspected cause, measure it, change the owner,
and repeat the same measurement. A broad subsystem pass also needs bounded discovery:

1. Compare maintained dependencies against the code, adapter glue and tests we would still own. For
   LSP decisions, start with [existing evaluations](lsp-dependency-decisions.md).
2. Measure an authored realistic fixture and a stress fixture. Separate startup, repeated warm
   operations, large outputs and recovery. Read the benchmark's setup and teardown.
3. Collect timings, sampled CPU stacks and allocation evidence. Include named live/peak heap and
   process RSS checkpoints where retention is relevant. State unavailable probes precisely.
4. Fix the simplest attributed cause first: unnecessary copies, repeated conversions or analysis,
   and avoidable collection growth. Additional caching or representations need measured benefit.
5. Verify diagnostic/event parity, IDs, ordering, snapshots and failure recovery. Inspect final
   callers and ownership; remove superseded code and concluded experiment scaffolding.
6. Remeasure the final implementation. Preserve raw results, commands, toolchain, fixtures, build
   identity and dirty source changes under `target/` or attached PR artifacts. Record conclusions,
   rejected options and concrete reopening triggers in the PR or the existing decision owner.

Criterion smoke (`--test`) only proves execution. A green latency budget does not discover CPU or
allocation hotspots. A noisy run supports a narrower claim, not a new threshold. Compare identical
toolchains, features and profiles; use alternating control/candidate runs where practical. Record
machine/OS and power/background-load conditions. Local laptop results do not establish release
trends or cross-platform rankings.

## Attribute CPU and heap costs

Build a focused executable first:

```sh
cargo bench --locked -p recite-benchmarks --bench sessions --no-run
```

Use the executable path Cargo prints, including its hash:

```sh
perf record --call-graph dwarf -- BENCH_BINARY --bench FILTER --profile-time 15
perf report
heaptrack BENCH_BINARY --bench FILTER --profile-time 15
valgrind --tool=massif BENCH_BINARY --bench FILTER --profile-time 15
```

These are external diagnostic tools, never production dependencies. Save captures under `target/`.
Profile mode repeats a workload without statistical timing analysis. Fixture setup still executes:
inspect stacks to distinguish preparation from the measured operation. Preserve build flags,
unresolved frames and lost-sample counts. If profiler permissions or tooling prevent a capture,
report that limitation; Criterion alone does not provide CPU attribution.

The user-facing `recite bench` command reports compiler, runtime and LSP project/fixture evidence.
The commands above expose maintainer investigations, including adapter and host costs that direct
runtime benchmarks do not cover.
