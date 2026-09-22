# Writer performance findings — 2026-09-18

The current GUI checkout was profiled without changing its production source or
restarting its windows. Two small candidates were implemented in a temporary copy
of the same working tree. These initial measurements describe the isolated experiments. The three accepted
optimizations were subsequently applied together; see the integration results below.

## Recommendation

Take the search-index cleanup and shared cold projection parse as one small
follow-up. Both stay within the authoring model, add no dependencies, preserve
public APIs, and require no persistent cache beyond the existing revision-owned
projection cache. Do not change the public AST representation in this pass.

| Operation | Before | Candidate | Change |
| --- | ---: | ---: | ---: |
| Index 100,000 passages | 331.53 ms | 282.88 ms | 14.7% faster |
| Search 100,000 passages | 0.0497 ms | 0.0196 ms | 60.6% faster; already cheap |
| Cold script projection, 500-passage document | 2.402 ms | 1.394 ms | 41.9% faster |
| Index allocation, 10,000 passages | 67.76 MB | 50.15 MB | 26.0% less |
| Search allocation, 10,000 passages | 9,674 bytes | 2,302 bytes | 76.2% less |
| Cold script projection allocation | 5.20 MB | 3.05 MB | 41.3% less |

CPU timings are medians from seven alternating baseline/candidate runs, optimized
builds, 200 documents of 500 passages. Each run contains five search samples.
The same experiment with cross-scene shared destinations showed 13.6% faster
indexing and 44.4% faster cold script projection.

Five full-workload Linux perf-stat runs retired 10,779,283,525 instructions before
and 10,202,091,811 after: **5.4% fewer instructions**. Mean elapsed time was 909 ms
before and 870 ms after, but relative timing variation was 2.4% and 5.3%; do not
claim a precise whole-app speedup from those elapsed measurements. This machine
was available for interactive manual testing during the measurements.

## What changes

### Search construction and query preparation

`apps/writer/crates/authoring/src/search.rs` currently concatenates all fields of
each passage, builds a temporary BTreeSet of its words, and then adds those words
to the posting lists. The candidate walks fields directly and appends a passage
ID only when it differs from that posting list's last ID. IDs are visited in
ascending source order, so this retains per-passage deduplication and sorted
postings without the temporary formatted string or set.

The query is also tokenized once in SearchIndex instead of once per document
shard. Unicode lowercase/token boundaries and AND semantics are retained.
Index construction drops from 495,128 allocations to 405,128 in the 10,000-passage
fixture; one search drops from 99 allocations to 23.

### Share the cold projection parse

`script.rs` parses and lowers the document, then asks for `passage_snapshot()`,
which parses and lowers it again when cold. The candidate passes the existing
parse/lowered source into the passage projector and populates the existing
revision-owned snapshot. No AST is retained after projection. Warm snapshots
still allocate nothing. Cold projection drops from 30,799 allocations to 19,931.

The affected modules remain cohesive: search owns postings; projection owns prose
ranges; projection_cache owns revision lifetime; script owns structural rendering.
Each experimental module remains below 250 lines.

## What is not an easy win

DHAT recorded about 391 MB cumulatively allocated during the original
10,000-passage workload, but only 15.06 MB peak live heap. This is allocation churn,
not evidence of a leak. Candidate peak live heap was 14.74 MB, a 2.1% reduction;
these changes should not be sold as a large resident-memory improvement.

Statement-vector growth accounts for about 115 MB of the original cumulative
allocation. The AST deliberately keeps large owned statement variants. Boxing
variants would affect a public representation and add indirection; pre-counting
statements would need to respect nested/recoverable grammar boundaries. Neither
is a suitable blind reserve-capacity tweak based on this five-lines-per-beat
fixture. Treat this as a separate investigation with diverse corpora.

Project opening and edit validation still include parsing/lowering and dependency
work. The candidate does not optimize those paths; edit timing fluctuated between
runs, so no edit-to-paint improvement is claimed. Saved-input cloning was already
cheap and warm script snapshots already allocated zero bytes.

Native rendering, GPU memory, font caches, idle CPU and hours-long retention were
not profiled. These model measurements do not establish GUI frame pacing.

## Evidence and validation

- Existing authoring-model suite: 45 tests passed on the experimental copy.
- Model Clippy: all targets and all features, warnings denied, passed.
- Fixed-corpus 10,000-passage heap gates passed for independent and shared-target
  topologies on the candidate.
- At the investigation checkpoint the patch applied cleanly and remained unapplied.
- These initial checks did not establish full integration or native GUI acceptance.

Local artifacts (temporary, not committed):

- `/tmp/recite-perf-heap-baseline/`: DHAT allocation stacks and operation report.
- `/tmp/recite-perf-cpu-baseline/`: perf sample stacks and build/environment metadata.
- `/tmp/recite-perf-paired/`: seven paired timing reports for each topology.
- `/tmp/recite-perf-before-stat.txt`, `/tmp/recite-perf-after-stat.txt`: counters.
- `/tmp/recite-perf-projection-heap.json`: combined candidate allocation report.
- `/tmp/recite-perf-experiment/`: isolated experimental source.
- `/tmp/recite-writer-perf-candidates.patch`: four-file implementation patch.

Reproduction uses the existing `just profile-writer cpu`, `just profile-writer
heap`, and the `large_project` benchmark with `--passages 100000`, default
`--per-document 500`, and optionally `--linked`. Run CPU measurements without
DHAT: allocator instrumentation changes timing substantially. Keep correctness,
allocation counts and CPU timing evidence separate.

## Integrated results — 18 September 2026

The search-index, shared cold projection parse, and conservative singleton-body
allocation changes are now applied together in the working tree. The rejected
boxing and indiscriminate small-capacity experiments are not included. No public
AST representation or persistent format changed; no dependency was added.

Five alternating baseline/combined runs per workload, pinned to CPU 2, used
100,000 original lines in 200 documents. Branching workloads add 40,000 replies.
The table reports median summed authoring-model operation times, not GUI frame
latency. Separate DHAT runs use 10,000 original lines and, for branching,
4,000 additional replies. MB is decimal cumulative allocated bytes.

| Workload | Model time ms, before → after | Indexing faster | Cold projection faster | Allocated MB, before → after |
| --- | ---: | ---: | ---: | ---: |
| linear-independent | 687.16 → 640.61 | 15.3% | 42.7% | 389.38 → 369.59 |
| linear-linked | 735.02 → 695.32 | 13.5% | 44.3% | 407.21 → 387.41 |
| branching-independent | 964.82 → 907.47 | 15.1% | 41.3% | 651.31 → 570.10 |
| branching-linked | 1041.49 → 979.31 | 15.7% | 44.6% | 676.14 → 594.93 |

Peak live heap fell about 2.1% for linear workloads and 2.4% for branching;
this remains predominantly an allocation-churn improvement. Opening and plain
edit timing are essentially unchanged. Search queries are 54–61% faster but were
already measured in hundredths of a millisecond. The parser's previously measured
comment-only tradeoff remains documented in the parser report.

Regression coverage checks duplicate search words across fields/shards, result
limits and source order, shared projection/cache freshness through edit and undo,
and singleton versus wider parser-body allocation. Existing heap gates now cap
index allocation at 55 MB and cold projection at 3.2 MB on the fixed linear
10,000-passage corpus, in addition to their previous peak/edit bounds.

`mise exec -- just check` passed with 1,455 workspace tests and 201 writer tests,
Clippy, editor/adapter checks, both heap gates, documentation, and benchmark smoke
checks. Four new regression tests are included in those totals. All eight touched
production modules remain below 200 lines; their existing ownership boundaries
remain intact. No native frame pacing, GPU memory, or long-session claim follows
from these model measurements.

The full gate required disk-backed scratch space outside the repository:
`TMPDIR=/home/mari/.cache/recite-optimisation-check-tmp`. The default `/tmp` ran
out of space during an isolated editor build, and repository-internal scratch
space was rejected by that test's target-path policy. Neither gate was weakened.
The successful log is `/tmp/recite-optimisations-check.log`.

Combined raw evidence and `combined-summary.txt` are in
`/tmp/parser-writer-validation/`; `*-combined-pair-*.json` contains the new paired
CPU samples. The experiment copy uses the same eight production modules as the
working tree, with only the temporary branching benchmark harness differing.
The application was not restarted, and no commit or push was made.
