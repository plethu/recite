# Profiling and optimisation

Performance work must justify its maintenance cost. A roughly 5% slowdown can be acceptable for a
substantial simplification; compare the workloads affected by the change and preserve semantics.

[Quality requirements](spec/quality.md) own performance policy. The paired LSP and workflow gates
have blocking regression budgets; other local measurements are investigation evidence until
[#109](https://github.com/plethu/recite/issues/109) establishes release baselines.

## Choose the measurement

Use `mise exec -- just perf …` without shell activation. Build before collecting timings and keep
other builds and tests out of the sampling window.

| Question                                       | Command                                                                   |
| ---------------------------------------------- | ------------------------------------------------------------------------- |
| Compiler phase, validation or encoding         | `just perf bench compiler tiny,small FILTER`                              |
| Direct runtime traversal or localisation       | `just perf bench runtime tiny,small FILTER`                               |
| Preview traversal, snapshots or retained trace | `just perf bench preview tiny,small FILTER`                               |
| In-process LSP analysis                        | `just perf bench lsp large,realistic:v1-pack FILTER`                      |
| Running LSP, protocol and scheduling           | `just perf compare BASE_COMMIT`                                           |
| Growing adapter history and preview replay     | `just perf bench sessions tiny FILTER`                                    |
| Adapter choices across source-file counts      | `just perf bench sessions tiny sessions/adapter_source_files/400`         |
| Real C ABI traversal, callbacks and encoding   | `just perf ffi FILTER`                                                    |
| Warm adapter and preview allocations           | `just perf allocations`                                                   |
| Warm C ABI allocations                         | `just perf ffi-allocations`                                               |
| Full compiler allocation churn                 | `cargo run --release -p recite-benchmarks --example compiler_allocations` |
| Build/watch refresh under fixture pressure     | `just stress watch`                                                       |
| Execution smoke, without performance claims    | `just perf smoke`                                                         |
| Paired Linux instruction and heap regressions  | `just perf regression BASE_COMMIT pr target/workflow-performance`         |

Use `just perf lsp --help` for bounded process/session probes. Its setup is `just perf setup`.
Criterion filters match group names, for example `lsp/change_refresh` or
`sessions/preview_condition_replay/deferred/2048`; `cargo bench -- --list` lists a target's cases.

`runtime/asset_decode` measures prepared `.recitec` bytes, including validation and asset
preparation. Use `RECITE_BENCH_SCALES` to select the compiler allocation example's existing
fixtures. Its counts exclude fixture preparation and result disposal; byte hashes check output
parity between runs.

The compiler Criterion target also exposes `compiler/compile_messagepack_with_schema`,
`compiler/pot_formatting` and `compiler/po_refresh_unchanged`. For example, run `just perf bench
compiler medium,realistic:v1-pack po_refresh_unchanged`. The payload case includes input cloning and
disposal of the unused compiled model. POT formatting uses an extracted template; PO refresh uses a
parsed catalogue already refreshed against that template. Their preparation and returned-value
disposal stay outside timing. POT extraction remains a separate case.

Batched phase benchmarks keep prepared inputs and receivers alive until timing stops. The CLI report
also excludes prepared-session setup and returned-value disposal. Its `build.measurement_revision`
rejects baselines from earlier timing boundaries; collect fresh baselines after this measurement
change instead of interpreting harness corrections as speedups.

The [session target](../crates/recite-benchmarks/benches/sessions.rs) owns warm-operation workload
shape and setup/teardown. `sessions/adapter_source_files` varies source-file count while keeping
history empty; its restored driver setup and returned-event disposal are outside timing. The
allocation example reports matching source-count cases alongside history and preview cases. The
[C ABI probes](../crates/recite-ffi/benches/) own callback/encoding parity checks and allocation
boundaries; inspect those owners when interpreting a result.

Keep allocation instrumentation separate from timing binaries. Allocated bytes/counts describe
churn, not live heap or process RSS. Size reports describe selected structures, not observed
allocations; use `memory_profile_report` or `id_memory_report` only for those questions.

## Writer workloads

The [Writer recipes](../apps/writer/justfile) expose project, headless UI, recovery and
retained-session and map-geometry workloads. The
[project benchmark](../apps/writer/crates/authoring/benches/large_project.rs) owns corpus shape,
operations, reports and fixed-corpus allocation bounds. For example:

```sh
mise exec -- just writer bench --passages 100000 --linked --output target/writer-linked.json
mise exec -- just writer bench --passages 10000 --per-document 10000 --output target/writer-single-scene.json
mise exec -- just writer bench-sessions 200000 100
mise exec -- just writer bench-map
mise exec -- just writer bench-catalogue 100000 20
mise exec -- just writer profile heap 10000 target/writer-heap
```

Project measurements include selective, common single-word, common multiword and expanded-result
search, plus whole-document Source draft capture and snapshot cloning. Search keeps exact counts and
source order; result snapshots retain their saved shards across replacements. The fixed
10,000-passage heap check bounds 100-hit searches at 20 KB and 10,000-hit expansion at 750 KB, and
requires allocation-free result clones and Source recovery capture/clones. Existing index,
projection, edit and peak-heap bounds remain in force. Applying Source drafts and serializing
recovery still process the full text.

Source completion queries the accepted kernel for unchanged bytes and prepares one independent
overlay for a changed draft, preserving accepted state and saved-document analysis. The popup and
keyboard share a result bound to the document identity, revision, draft and cursor. Sidebar captions
scan authored END destinations without classifying the full flow graph; unchanged ordered names and
badges share captions across prose edits.

Compiler completion and hover sites use the snapshot's line index and compute replacement spans
within the selected line. Contextual speaker metadata walks preceding indexed lines only as far as
the nearest beat header, preserving explicit overrides and malformed-context handling. Project
validation stops expanding contexts once they contain every current document. Unchanged ordered
exports retain their definitions membership while dependencies and facts still update. Stable-ID
collision checks retain source order; the separate line-ID index is built only for eligible choice
echo lookups.

The project workload also measures a warm manuscript prose edit followed by its Script projection.
Prose validation remains parser-owned; accepted edits reuse their checked passages and, when Script
was already requested, construct it from that same transient parse. Raw source edits and undo/redo
invalidate those revision-bound projections. This headless operation excludes native input and
paint.

For prose edits inside one restart region, identical surrounding region bytes and ordered frozen IDs
allow checked outputs to be reused with relocated coordinates. Uncertain mappings and plural-bearing
Script projections fall back to whole-file projection. A single-region file still needs a whole-file
parse; materializing the public passage and Script collections remains linear in their size. Passage
clones share immutable content while keeping coordinates specific to each revision. An owned copy
uses copy-on-write when its content is changed; retained snapshots remain independent.

The session workload runs discovery/indexing, first visits and edits, revisits with undo/redo,
save-all and explicit closing. It checks preserved prose/history and released session owners and
reports RSS, peak RSS and thread checkpoints where the host exposes them. RSS can retain allocator
pages after closing; it is not a live-heap measurement. Recovery timing queues full-document Source
drafts and verifies the latest durable snapshot after reopening. These workloads are headless and
opt-in; recovery reports draft buffer preparation separately from snapshot queuing and durable
flush. Limit build jobs and run measurements sequentially on a shared machine; record process
resource limits and background load with the evidence.

Catalogue workloads cover an authored scene and generated Unicode PO files, draft recovery capture,
Save All, and exact-count substring filtering with a populated current scene. Index preparation and
replacement are reported separately from warm queries; reconstruction retains the old index until
the replacement is ready. The open queue retains one normalized index keyed by saved catalogue and
passage snapshots. Draft revisions update matching without rebuilding saved fields, and paging or
focus changes reuse results. Recovery shares one original source buffer and immutable draft values;
capturing a checkpoint still copies draft IDs and handles, while cloning a checkpoint is cheap.
Serialization and full final-source validation remain separate work. These probes exclude native
painting and real input and do not establish the release baseline.

Map geometry timing covers authored, linear and high-fan-out graphs separately from painting.
Topology is memoized independently of camera and hover. Content refresh derives current connections
and labels, and reuses geometry when ordered beat IDs and entry flags, ordered connection endpoints
and return classifications, and saved placement remain identical. Other changes reconstruct layout
and routes. The probe measures both reconstruction and content refresh after an accepted edit,
including connection derivation and guard work; it excludes native frame pacing.

Kernel-open timing excludes filesystem discovery, indexing and painting. The
[profile script](../scripts/profile-writer.sh) records builds/environment, refuses overwrite and
runs perf or bench-only DHAT instrumentation; instrumented latency is not normal-run timing and
production allocation is unchanged. Allocation bounds run in `just writer check`; investigate
unexplained regressions before changing them. Native scale requirements belong to
[Writer acceptance](../apps/writer/acceptance.md#scale-and-performance).

## Bounded regression checks

Install Valgrind with the host package manager and run `just perf regression-setup` for the pinned
[Gungraun](https://github.com/gungraun/gungraun) runner. The private `tools/recite-perf` Rust crate
uses upstream measurement, baselines, limits and typed summaries. The shell recipe builds the base
and candidate before measurement and applies the current harness and fixtures to both. It preserves
the original and resolved control locks; dependency resolution for harness additions runs offline
after both locked dependency sets are fetched. Preparation and receiver/result disposal are
excluded. DHAT filters allocation stacks to the operation; its peak counter is operation-attributed
bytes live at the whole-process heap peak, not an isolated operation peak or process RSS.

The three runtime step cases each process 64 independent prepared sessions. Their session clones and
output buffers are prepared outside counters; batching dilutes one-off allocator housekeeping.
Compare both revisions with the same current harness rather than reusing older single-step counts.

`pr` checks eight compiler/runtime/preview operations on small and authored realistic fixtures, plus
a prepared 64-driver adapter choice batch with 400 source files. The adapter case excludes fixture
construction and session startup, then checks exact events and serialized snapshots outside
counters. `full` adds medium fixtures. Both include cold effect-heavy Writer script projection at
1,000 and 10,000 entries, validating Unicode prose, authored IDs, effects and order outside the
measured operation, four accepted/changed draft completion cases on wide and dense projects, and a
64-session bound-text prompt batch using the checked language-pressure fixture. Both also cover
JSON/TOML schema loading, schema-backed workspace forks, LSP completion/definition/references and
diagnostic publication on wide/dense projects, and PO imports into empty/populated catalogues.
Schema-action responses, shared static/parameterized UI labels, and dialogue edits with saved/open
schemas have fixed-input cases too. Prepared completion replacements are checked separately on
wide/dense projects, with query preparation and result validation outside counters. `writer` and
`authoring` run just their respective targets. Use a fresh output directory. Limits belong to
[the shared configuration](../tools/recite-perf/benches/support/budget.rs); elapsed time remains a
Criterion investigation. Missing paired measurements fail closed, including Gungraun's automatic
creation of a missing named baseline.

CI runs this lane only for measured Rust/model inputs, fixtures and shared build changes. Weekly and
release runs use `full` and `just stress bounded` (large CLI, tiny/small watch recovery). The
output's `evidence/` contains raw profiles, summaries, environment, binaries' hashes, locks, current
harness and dirty source changes. The instruction/heap gate covers Linux headless workflows; it does
not replace the existing LSP process gate or native Writer acceptance evidence in issue #109. CI
keeps fresh evidence outside its build cache; `RECITE_PERF_BUILD_DIR` selects reusable build storage
without reusing profiles or baselines.

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
