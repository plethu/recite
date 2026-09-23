# Writer scalability

The writer bounds much of its visible work, shares saved source text, and indexes
project search. The authoring kernel retains compact validation facts and updates
only changed documents and affected dependents. Single-line, multiline and ID edits
now avoid project-wide validation in the generated million-passage workload.
[The profiling pass](benchmarks/2026-09-16-dependencies/README.md) records latency,
heap ownership, RSS, regression checks and the remaining limits. Freya remains the
selected UI; these model measurements do not establish native GUI readiness.

## Implemented boundaries

- **Data and computation:** saved document clones share immutable source bytes.
  Passage and script projections are cached until source changes; passage lookup
  uses frozen IDs. Graph topology, adjacency, layout and routes are cached apart
  from camera/hover state. Scene switching reuses the discovered project context;
  Refresh explicitly discovers new files and updates saved context.
- **Bounded rendering:** scene navigation, project results and connections use
  fixed-height virtual rows. Map cards and routes are culled against the viewport
  with overscan, including routes whose endpoints are offscreen. The selected
  card stays mounted. Scenes over 200 beats default to an explicitly labelled
  neighbourhood of at most 80 connected beats; Whole scene is an explicit choice.
- **Finding and reading:** project search indexes saved dialogue/reply text,
  speaker, document and beat. Queries match all complete words, case-insensitively;
  results retain document/beat context and show at most 100 matches plus the total.
  Save reindexes the affected document; Refresh rebuilds the saved index. Search
  does not silently include uncommitted drafts. Selecting a result opens its beat.
  The scene drawer also filters names. Back/forward retains up to 128 visited beats
  within a scene. One pinned, labelled snapshot supplies reading context while
  navigating; long reference previews are explicitly truncated.
- **Sustained writing:** prose is paged in groups of 32 entries without recycling
  active text editors or flattening condition groups. Page navigation applies a
  valid draft before leaving it and retains undo. History stores UTF-8 replacement
  transactions with a 32 MiB retention budget, always retaining the latest edit
  even if that single transaction exceeds the budget. This is per-document undo,
  not a durable cross-project history.
- **Recovery and cold loading:** one recovery worker owns the file lock and disk
  writes. Pending snapshots coalesce; failures are visible and retryable. Save and
  Keep recovery and close wait for a durable flush. Initial project open runs on
  a worker, reports progress, and discards cancelled results or results arriving
  after the current text changed. Cancellation discards the result; it does not
  interrupt compiler work. Subsequent scene switches, refresh and committed edits
  still have synchronous kernel work.
- **Resizing:** pane dividers move a guide while dragging and commit width on
  release. Text stays at its existing measure during the drag; Escape cancels.

## Repeatable workloads

Run from the repository root:

```sh
mise exec -- just bench-writer --passages 10000 --output /tmp/writer-10k.json
mise exec -- just bench-writer --passages 100000 --output /tmp/writer-100k.json
mise exec -- just bench-writer --passages 1000000 --output /tmp/writer-1m.json
mise exec -- just bench-writer-ui 1000
mise exec -- just bench-writer-ui 10000
mise exec -- just bench-writer-recovery
```

The project workload defaults to 500 passages per document and five passages per
beat. It generates unique frozen IDs, Unicode prose, speakers and linear links.
It measures indexing, warm searches, cloning saved inputs, opening an authoring
kernel with the full project context, cold/cached projection, ten single-passage
edits, ten undos and ten redos, then ten multiline and ID edits with undo.
`--linked` makes all documents link to one shared destination. It reports source bytes, estimated retained history
allocation and Linux process peak RSS. Opening here excludes filesystem discovery,
index construction and painting: those are not one end-to-end project-open timing.

The GUI workload is a separate 1,000/10,000-beat conversation. It counts mounted
cards, measures initial headless settling and 30 actual keyboard selection changes,
and verifies that selection changed. The recovery workload queues 100 drafts in a
10,000-passage document, flushes and verifies the latest snapshot after reopening.
These generated sources contain no game dialogue or proprietary assets.

## Original diagnostic-heavy results, 16 September 2026

The original generator repeated block names and default markers across documents.
Those inputs caused project diagnostics: the numbers below describe that historical
diagnostic-heavy workload, not a clean project. The generator has been corrected,
its harness now asserts zero initial diagnostics, and the following kernel comparison
reruns both versions against identical corrected sources. Original generator code
is retained in commit `df12b87dc8071a1eb825cfd18be0e3329b291973`.

AMD Ryzen AI 7 350, Linux x86_64, Rust 1.96.0. Project timings use the optimized
bench profile; GUI and recovery timings use the unoptimized test profile.
This is a development-machine sample, not an isolated performance lab or a
cross-platform acceptance claim. [Raw measurements](benchmarks/2026-09-16/README.md)
include every sample and commands. Medians below use five searches and ten edits;
with so few samples, the maximum is more useful than claiming a stable tail latency.

| Project passages / documents | Search median / max | Edit median / max | Kernel cold open | Process peak RSS |
| --- | --- | --- | --- | --- |
| 10,000 / 20 | 0.004 / 0.014 ms | 15.5 / 16.1 ms | 38.6 ms | 44.4 MiB |
| 100,000 / 200 | 0.049 / 0.267 ms | 222.9 / 230.6 ms | 406.4 ms | 406.8 MiB |
| 1,000,000 / 2,000 | 1.11 / 3.34 ms | 2,418 / 2,617 ms | 4,105 ms | 3.86 GiB |

The million-passage corpus contains 122,185,364 source bytes. Building its search
index took 3.42 seconds. Cloning its saved-document handles took 0.063 ms median;
a cached active-document script snapshot took under 0.001 ms. Ten small edits
retained an estimated 580 bytes of history in each workload, excluding container
spare capacity, the current document, compiler state and caches. Undo/redo still
pay kernel validation costs: the million-passage redo median was 2.71 seconds.
Peak RSS is for the entire benchmark process; it does not identify which subsystem
owns each allocation, and it is not a native GUI memory measurement.

| Conversation beats | Initially mounted cards | First headless view | Navigation median / p95 / max |
| --- | --- | --- | --- |
| 1,000 | 3 | 327 ms | 8.90 / 9.62 / 9.79 ms |
| 10,000 | 3 | 1,501 ms | 21.90 / 23.41 / 24.29 ms |

Here p95 is nearest-rank over 30 samples. These measure test-runner event/layout
work, **not native frame pacing, text-input latency or GPU rendering**. Recovery
queue submissions were below 0.05 ms in this run; the final durable flush took
129 ms and reopened the latest draft correctly.

## First kernel comparison on a valid project

[Before/after measurements](benchmarks/2026-09-16-kernel/README.md) use the same
host and optimized bench profile, with unique block names/IDs and one default.
The baseline is the compiler at `df12b87d`; both runs use the corrected generator
and extended harness. Every initial snapshot has zero diagnostics.

| Passages | Edit median before / after | After edit max | Multiline edit after | ID change after |
| --- | --- | --- | --- | --- |
| 10,000 | 7.01 / 1.39 ms | 1.73 ms | 11.23 ms | 9.45 ms |
| 100,000 | 90.39 / 2.03 ms | 2.46 ms | 91.02 ms | 93.53 ms |
| 1,000,000 | 1,007 / 14.01 ms | 15.92 ms | 1,040 ms | 1,059 ms |

Edits have ten samples; multiline/ID changes have one sample each, followed by
undo. Million-passage undo/redo medians are 13.97/13.93 ms. The corrected corpus
contains 123,754,976 bytes. Peak RSS before/after is 3,183,692/3,183,644 KiB
(about 3.04 GiB), and cold kernel opening is 3.30/3.38 seconds. Those are effectively
unchanged, not memory or cold-start wins. Retained history is 676 estimated bytes
after the extended transaction sequence. These are model timings, not native
edit-to-paint measurements.

## Incremental project validation and profiling

Local validation runs against the parsed document, then discards its full AST.
Project facts retain only identities, block definitions/defaults, references, echo
targets, source locations and recovery participation. Saved inputs, analyses and
snapshots share source bytes. A reverse dependency index tracks definitions and
consumers separately, so scenes sharing a destination do not form one giant
revalidation group. Moving that destination's location does not change whether
its blocks resolve; changing/removing its block definitions invalidates callers.

Changed documents and affected dependents are validated with their required
providers using the existing project validator. Only each target's diagnostics
are published: context providers may omit their own unrelated dependencies.
Completeness and global stable-ID recovery transitions conservatively refresh
all documents. Schema changes create a new kernel. The uncached batch validator
remains the differential oracle, including related diagnostic locations.

```sh
mise exec -- just profile-writer cpu 100000 /tmp/writer-cpu
mise exec -- just profile-writer heap 10000 /tmp/writer-heap
mise exec -- just check-writer-heap
mise exec -- just bench-writer --passages 1000000 --linked --output /tmp/writer-linked.json
```

The profiler builds the existing optimized benchmark with line debug information,
then runs the executable directly under Linux `perf` (DWARF stacks) or DHAT. It
records environment/build metadata and refuses to overwrite a prior profile.
DHAT is a development dependency, with its allocator enabled only in this bench
under `heap-profile`; production allocation is unchanged. Instrumented reports
include per-operation allocation counts/bytes and live/peak heap bytes. Their
latencies and RSS are not compared with uninstrumented runs.

`check-writer` (and therefore `just check`) runs both independent and shared-target
10,000-passage heap checks. Bounds are 20 MB peak live heap, 55 MB allocated for
indexing, 3.2 MB for cold script projection, and 4 MB allocated per
wording/undo/redo/multiline/ID edit, using 500 passages per document. These are fixed
corpus allocation contracts with measured headroom, not machine timing budgets.
A private work-count test additionally prevents unrelated callers being revalidated;
batch-equivalence tests cover joins, splits, removals, defaults, recovery, echo/ID
collisions and context-only diagnostics. Keep bounds reviewed alongside raw heap
evidence rather than increasing them to accommodate an unexplained regression.

## Remaining work and acceptance

Project discovery/indexing and cold validation still scale with corpus size.
A high-fan-out semantic change legitimately invalidates all its dependents; global
completeness/recovery changes can still require a full refresh. Context projection
can also be expensive for a single enormous file or many colliding definitions.
Source summaries and saved-search postings remain sizeable heap owners. Large-scene
topology invalidation is still scene-wide, and Source view opens the whole document.

Before claiming RPG production scale, extend these workloads to branching/fan-out,
cycles, cross-scene references, long prose, diagnostics-heavy projects and prolonged
editing. Existing functional tests cover cycles, convergence, conditions, rejected
edits, Unicode undo, recovery failure and conflicts; that is not performance
coverage for all those combinations. Measure end-to-end project discovery/open,
scene switching, edit-to-paint, search-to-passage reveal, memory over hours, crash
injection and physical Linux/macOS input/rendering. Screen-reader navigation across
virtual rows and IME composition remain manual acceptance requirements.

Proposed native targets remain 16.7 ms camera/guide frames, p95 edit-to-paint below
50 ms, and warm search results below 100 ms on named hardware. Current results do
not establish those targets. Keep the writer centred on a conversation and nearby
context; project-wide production navigation can add authored folders/tags without
inventing a hierarchy from graph connectivity.

## Search and projection measurements

The September 18 pass removed temporary search-word collections, shared the cold
projection parse, and avoided unused statement slots in singleton parser bodies.
No public AST or persistent format changed. Five alternating baseline/combined
runs per workload used 100,000 lines in 200 documents, pinned to CPU 2. Branching
workloads added 40,000 replies. These are median summed model-operation times,
not GUI frame latency; separate DHAT runs measured 10,000 lines.

| Workload | Model time ms, before → after | Cumulative allocated MB, before → after |
| --- | ---: | ---: |
| Linear, independent | 687.16 → 640.61 | 389.38 → 369.59 |
| Linear, linked | 735.02 → 695.32 | 407.21 → 387.41 |
| Branching, independent | 964.82 → 907.47 | 651.31 → 570.10 |
| Branching, linked | 1041.49 → 979.31 | 676.14 → 594.93 |

Peak live heap fell roughly 2–2.4%; the main gain was lower allocation churn.
Opening and plain-edit timings were essentially unchanged. The parser-only
comment workload regressed about 7% (0.14 ms per 10,000 comments), so this is
not a blanket parser speedup. Keep CPU and DHAT runs separate.
The 10,000-passage heap gate caps index allocation at 55 MB and cold projection
at 3.2 MB, alongside the peak/edit bounds. Native frame pacing, GPU memory and
long-session use still require separate measurements.
