# Parser allocation investigation — 2026-09-18

The statement-allocation hotspot is real. The smallest candidate reserves one
slot for a body known to contain only one statement, instead of accepting Vec's
initial four-slot allocation. It preserves the public AST and parser semantics.
This experiment was subsequently applied with the search and projection changes;
see the integration results in [the writer report](writer-performance-findings.md).

## Decision

The complete writer-model benchmark now validates this as a modest allocation
cleanup: branching workloads allocate 8.5–8.8% less over the full operation
sequence, while peak live heap is unchanged. Total elapsed time remains within
about 1% of baseline. Retired instruction counts rise slightly rather than fall.
It is suitable as a memory-churn follow-up, not a responsiveness claim or an
urgent GUI milestone requirement. Prioritize the search/projection changes in
[the writer report](writer-performance-findings.md), which have clearer CPU gains.

The earlier comment-only lowering regression remains a known tradeoff: about 7%
(0.14 ms per 10,000 comments). The full writer workloads do not erase that result.

Do not box the public Choice variant in this milestone. That alternative saves
memory in many corpora but increases retained memory in choice-heavy input and
requires a public Rust API change.

## Cause and candidate

On this x86_64 build, Statement and Choice are each 856 bytes; Line is 416,
Comment 64, and the remaining variants are 168–240. A vector's initial four
statement slots occupy 3,424 bytes even when only one is used. Nested singleton
bodies multiply that waste. Choice target bodies also allocate a statement vector
that is consumed while lowering the target, producing transient allocation churn.

The candidate checks only an empty statement vector, after successfully lowering
its first statement. It skips subsequent blank lines and reuses BodyCursor's
existing boundary predicate. At EOF or the body's boundary it reserves exactly
one slot; otherwise ordinary growth remains. The check does not advance the
cursor, emit diagnostics, or skip recovery. It is deliberately conservative about
malformed trailing content.

One helper in Lowerer owns the allocation policy. Existing root, prose-body, and
statement-body push sites call it. The boundary predicate gains only crate-local
visibility. No cache, dependency, public type, serialized format, ID, or traversal
order changes. All four touched modules remain below 200 lines and retain their
existing responsibilities.

## Measurements

These measure **lower_source_file only**, with parsing and source generation
outside the measured region. MB means decimal cumulative allocated bytes or
bytes still live with the lowered result; neither is application RSS. CPU builds
have no DHAT instrumentation. Five alternating baseline/candidate invocations
were pinned to CPU 2, each with seven samples per case; timings below are pooled
medians. The host remained available for interactive testing, so small timing
changes should not be treated as precise improvements.

| Corpus | Allocated MB, before → after | Retained MB, before → after | Median ms, before → after | Time change |
| --- | ---: | ---: | ---: | ---: |
| lines-1 | 49.45 → 23.77 | 38.61 → 12.93 | 11.207 → 9.244 | -17.5% |
| lines-5 | 28.93 → 28.93 | 15.24 → 15.24 | 8.768 → 8.441 | -3.7% |
| lines-8 | 21.08 → 21.08 | 10.09 → 10.09 | 8.201 → 8.028 | -2.1% |
| lines-256 | 24.26 → 24.26 | 9.69 → 9.69 | 7.519 → 7.368 | -2.0% |
| lines-10000 | 35.30 → 35.30 | 15.14 → 15.14 | 7.909 → 7.551 | -4.5% |
| choice-5 | 64.11 → 38.43 | 15.18 → 15.18 | 9.977 → 9.109 | -8.7% |
| comment-5 | 23.21 → 23.21 | 14.34 → 14.34 | 1.983 → 2.127 | +7.3% |
| branch-5 | 108.59 → 57.23 | 84.08 → 32.72 | 17.924 → 15.078 | -15.9% |
| long-prose | 10.47 → 10.47 | 5.30 → 5.30 | 1.297 → 1.262 | -2.7% |
| nested-16 | 36.85 → 15.02 | 29.58 → 7.75 | 6.224 → 5.591 | -10.2% |
| nested-wide-5 | 32.19 → 32.19 | 16.55 → 16.55 | 7.816 → 7.734 | -1.1% |
| nested-wide-8 | 23.93 → 23.93 | 11.38 → 11.38 | 7.360 → 7.398 | +0.5% |
| nested-wide-256 | 23.77 → 23.77 | 9.64 → 9.64 | 6.007 → 5.766 | -4.0% |
| malformed | 25.26 → 14.99 | 18.67 → 8.40 | 6.125 → 5.635 | -8.0% |

Line/comment/choice cases contain 10,000 statements, grouped into bodies of the
named width. Choices have a nested END target. Branch cases contain 10,000
if/else structures with nested statements. Nested-16 uses 500 sixteen-level
chains. Nested-wide cases put 10,000 lines into branch bodies of varying width.
Long-prose uses 1,000 paragraphs; malformed uses 2,000 branches with indentation
and else-tail errors. Wider bodies have exactly unchanged measured allocations.

## Alternatives measured

Starting every nested vector at capacity one was too indiscriminate: it increased
cumulative allocation by 5.3% for five-statement nested bodies and 4.5% for eight.
It was rejected in favour of the terminal-body check.

Passing the 856-byte statement into the helper by value was also unnecessary;
the retained candidate reserves capacity and leaves each push at its original
site. A forced-inline variant did not remove the comment-only slowdown and showed
no consistent overall CPU advantage, so that annotation was not retained.

Boxing only Choice reduced Statement from 856 to 416 bytes. It cut cumulative
allocation by 30.6–45.5% in several short-line/comment/choice corpora, but retained
memory in the choice-heavy corpus rose from 15.18 MB to 16.70 MB (+10.0%). Each
choice then needs both its vector slot and its box. It also changes public enum
construction and requires compiler consumer adjustments. This is a separate
representation decision, not a free allocation-policy fix.

## Correctness and reproduction

Full lowered-result Debug fingerprints match baseline for all 14 generated cases
and 15 checked-in valid/invalid fixtures, including statements, spans, recovery
classification, diagnostics, and their order. Capacity itself is intentionally
excluded from those semantic fingerprints. This is differential evidence rather
than a proof for all inputs.

The retained candidate passes 313 parser/compiler tests (including doctests) and
Clippy for both crates with all targets and warnings denied. The patch passes
`git apply --check`; the report passes typos; Git policy and `git diff --check`
also pass. The full workspace/GUI integration gate was not rerun for an unapplied
experiment. Test and Clippy logs are `/tmp/parser-reserve-{tests,clippy}.log`.

The candidate was tested in `/tmp/recite-parser-experiment`, copied from the dirty
working tree, without modifying production source or restarting GUI windows.

Local artifacts:

- Candidate patch: `/tmp/recite-parser-singleton-reserve.patch`.
- Temporary harness: `apps/writer/crates/authoring/benches/parser_alloc.rs` in that copy.
- Build/run driver: `/tmp/run-parser-profile.py`; takes a variant name.
- Heap reports: `/tmp/parser-{baseline,singleton-reserve,small-body,boxed-choice}-heap.json`.
- Paired CPU reports: `/tmp/parser-paired-reserve/`.
- Rejected inline comparison: `/tmp/parser-paired-inline/`.

The driver builds optimized heap-profile and uninstrumented benchmarks separately.
A saved CPU binary accepts an output JSON path and the `fixtures/recite` directory.
For example:

```sh
taskset -c 2 /tmp/parser-singleton-reserve-cpu /tmp/parser-repeat.json "$PWD/fixtures/recite"
```

These temporary artifacts are local investigation evidence, not a committed
benchmark API. Before adopting the patch, retain a suitable regression benchmark
in the repository and run the integration gate on the combined changes.

## Complete writer-model validation

The follow-up compared the parser patch alone against current production source;
it did not include the earlier search/projection candidates. Both builds used the
same benchmark harness and corpus. No production source or running GUI changed.

The existing large_project benchmark exercises search indexing/querying, opening
and validating a project document, cold/warm script projection, ten edits,
undo/redo, multiline edits, and stable-ID changes. Two topologies were run:
independent destinations and destinations shared across documents. The branching
variant replaces each beat's terminal divert with two nested replies to that
same destination. It adds 40% more passages; comparisons are always within the
same corpus, never against the smaller linear corpus.

CPU: 100,000 original lines in 200 documents; branching has 40,000 additional
replies. Seven alternating baseline/candidate runs per topology, pinned to CPU 2,
uninstrumented optimized binaries. The table reports median summed operation
times per invocation, not startup-to-exit or GUI frame latency.

Heap: 10,000 original lines, 20 documents; branching adds 4,000 replies. Numbers
are summed per-operation DHAT allocations. Peak live heap is the maximum reported
live heap across the run. MB is decimal.

| Workload | Allocated MB, before → after | Reduction | Peak live MB, before → after | Operation time ms, before → after |
| --- | ---: | ---: | ---: | ---: |
| linear-independent | 389.38 → 389.38 | 0.0% | 15.06 → 15.06 | 845.47 → 838.44 |
| linear-linked | 407.21 → 407.21 | 0.0% | 15.07 → 15.07 | 906.82 → 899.72 |
| branching-independent | 651.31 → 593.78 | 8.8% | 22.32 → 22.32 | 1174.91 → 1172.26 |
| branching-linked | 676.14 → 618.62 | 8.5% | 22.32 → 22.32 | 1246.20 → 1254.39 |

For independent branching dialogue, allocation fell 9.7% during indexing, 8.6%
during opening, 11.3% during cold projection, and 10.2% during a plain edit.
Transient target-body vectors explain why cumulative allocation falls while
retained/peak memory does not. Linear workloads have exactly unchanged operation
allocation counts.

Three alternating perf-stat pairs per workload provide a less timing-sensitive
CPU check: retired instructions increased by 0.28% for linear independent,
0.044% for branching independent, and 0.030% for branching shared destinations.
This is not a CPU optimization. Individual operation timings move both ways;
no consistent responsiveness improvement is established.

All 45 authoring-model tests pass on the candidate (`model-tests.log`).
Both standard linear topologies pass the existing fixed-corpus heap gates.
The expanded branching baseline itself exceeds the original 20 MB live-heap
budget (22.32 MB), so branching evidence uses direct comparison rather than
claiming to pass a budget calibrated for 10,000 total passages. Both versions
validate with zero diagnostics, retain the same history-byte counts, and pass the
benchmark's search-size, cache-identity, and undo/redo assertions.

Artifacts are in `/tmp/parser-writer-validation/`: saved CPU/heap binaries, build
logs, seven timing reports per version/topology, heap reports and branching DHAT
stacks, perf-stat CSVs, and `summary.txt`. The temporary reproduction drivers are
`/tmp/validate-parser-writer.py` and `/tmp/validate-parser-writer-branching.py`;
the latter runs the expanded corpus without the inapplicable fixed-corpus gate.
`RECITE_BENCH_BRANCHING=1` selects the branching corpus only in this temporary
harness. These tests cover the authoring model; native rendering, GPU memory,
idle CPU, and long-session retention remain outside the measurements.
