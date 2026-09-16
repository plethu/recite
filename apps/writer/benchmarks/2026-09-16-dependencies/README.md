# Incremental validation and heap profiling, 16 September 2026

AMD Ryzen AI 7 350, Linux x86_64, Rust 1.96.0. Optimized bench profile with line
debug information. The baseline compiler is signed commit `ab18f5cb733d`; a
disposable checkout used that compiler with the same current harness and profiling
configuration. Sources contain generated dialogue only. Each normal run asserts
zero initial diagnostics, with 500 passages per document and five per beat.

Normal timing runs are sequential and separate from profiler instrumentation.
Each edit kind has ten samples; reported maxima are observed maxima, not stable
percentile estimates. The shared-destination case makes every scene return to one
hub, and edits that hub; it tests that location changes do not invalidate callers.
It does not represent every possible RPG project shape or native UI work.

## Results

| Passages | Multiline median before / after | ID median before / after | Peak RSS before / after |
| --- | --- | --- | --- |
| 10,000 | 6.62 / 1.41 ms | 6.89 / 1.43 ms | 0.035 / 0.021 GiB |
| 100,000 | 79.86 / 2.27 ms | 83.77 / 2.24 ms | 0.307 / 0.152 GiB |
| 1,000,000 | 907.21 / 13.50 ms | 938.82 / 13.39 ms | 3.036 / 1.460 GiB |

The [shared-destination million-passage run](after-linked-1000000.json) measured 14.56 ms multiline and 14.39 ms ID-edit medians, with 1.461 GiB peak RSS.

The adjacent `before-N.json` and `after-N.json` reports retain all samples.

## Profiling evidence

CPU samples use Linux `perf record --call-graph dwarf` against the benchmark
executable, not Cargo. [Before](before-perf.txt.gz) and [after](after-perf.txt.gz) reports
are gzip-compressed text covering 100,000 passages. Sampling identifies hot work; percentages are not speedup
ratios. Stable-ID validation and source-tree traversal dominated validation samples
before the change. The new kernel retains compact facts, projects only required
validation context and publishes only the target document's diagnostics.

DHAT profiles cover 10,000 passages with the same operation sequence. The
[before](before-heap-sites.json) and [after](after-heap-sites.json) largest allocation
sites include full stacks. Retained statement vectors were the largest baseline
site. The implementation removes retained ASTs from kernel analyses, shares source
buffers with saved inputs/snapshots and shares document identities in the reverse
index. Source summaries, search postings and compact facts remain significant.
Peak live heap fell from 29,692,228 to 15,061,942 bytes. The maximum multiline-edit
allocation fell from 5,127,889 to 3,742,111 bytes on this fixed corpus.

[Before](before-heap-10000.json) and [after](after-heap-10000.json) instrumented
reports retain per-operation allocated bytes, allocation counts and live heap.
Instrumented timings and RSS include profiler overhead; do not compare them with
normal timing/RSS results. Compressed raw [before](before-dhat.json.gz) and
[after](after-dhat.json.gz) DHAT traces can be decompressed and opened in the
[DHAT viewer](https://nnethercote.github.io/dh_view/dh_view.html).

## Reproduction and regression checks

```sh
mise exec -- just bench-writer --passages 1000000 --output /tmp/writer-normal.json
mise exec -- just bench-writer --passages 1000000 --linked --output /tmp/writer-linked.json
mise exec -- just profile-writer cpu 100000 /tmp/writer-cpu-new
mise exec -- just profile-writer heap 10000 /tmp/writer-heap-new
mise exec -- just check-writer-heap
```

The profiler recipe saves Cargo build metadata, revision/dirty-state information,
platform/toolchain, the workload report and profiler output in a fresh directory.
`perf` must be installed and permitted by the Linux host. DHAT is bench-only and
portable; its profiling mode is kept separate from its allocation-testing mode.
This follows the [Rust Performance Book's profiling guidance](https://nnethercote.github.io/perf-book/profiling.html)
and [DHAT's profiling/testing instructions](https://docs.rs/dhat/0.3.3/dhat/).

The normal writer gate runs both independent and shared-destination allocation
checks: under 20,000,000 peak live bytes and under 4,000,000 allocated bytes for
each wording/undo/redo/multiline/ID edit on the fixed 10,000-passage corpus. These
bounds have headroom above the measured result but reject retained full-project
ASTs and project-wide validation allocations. They are not timing promises.
Compiler work-count tests check bounded revalidation directly, and differential
tests compare full diagnostics with uncached batch validation after edits,
removals, recovery and completeness transitions. Changing a referenced block still
invalidates all callers; moving its location does not.

The model still walks document-level snapshot bookkeeping. Huge single files,
many colliding definitions and genuine high-fan-out semantic changes can require
substantial work. Global completeness/recovery transitions can refresh everything.
Cold load, long editing sessions and native edit-to-paint remain separate acceptance
work; see the [scalability report](../../scalability.md).
