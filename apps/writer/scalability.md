# Writer scalability

The writer bounds much of its visible work, shares saved source text, and indexes project search.
The authoring kernel retains compact validation facts and updates only changed documents and
affected dependents. Single-line, multiline and ID edits now avoid project-wide validation in the
generated million-passage workload. Repeatable workloads and heap checks below cover model latency,
allocation, and regression limits. Freya remains the selected UI; model measurements do not
establish native GUI readiness.

## Measured boundaries

The [application guide](guide.md) owns navigation and editing behavior. These probes exercise the
model and bounded rendering, not a whole native authoring session. Source bytes and accepted
analysis are shared across document sessions; closing a tab releases its retained history and
workers. Project loading has cooperative cancellation, including compiler and search checkpoints. An
individual filesystem operation or parse finishes before its next checkpoint. Scene switches,
refresh and committed edits still perform synchronous kernel work.

## Repeatable workloads

Run from the repository root:

```sh
mise exec -- just writer bench --passages 10000 --output /tmp/writer-10k.json
mise exec -- just writer bench --passages 100000 --output /tmp/writer-100k.json
mise exec -- just writer bench --passages 1000000 --output /tmp/writer-1m.json
mise exec -- just writer bench-ui 1000
mise exec -- just writer bench-ui 10000
mise exec -- just writer bench-recovery
```

The project workload defaults to 500 passages per document and five passages per beat. It generates
unique frozen IDs, Unicode prose, speakers and linear links. It measures indexing, warm searches,
cloning saved inputs, opening an authoring kernel with the full project context, cold/cached
projection, ten single-passage edits, ten undos and ten redos, then ten multiline and ID edits with
undo. `--linked` makes all documents link to one shared destination. It reports source bytes,
estimated retained history allocation and Linux process peak RSS. Opening here excludes filesystem
discovery, index construction and painting: those are not one end-to-end project-open timing.

The GUI workload is a separate 1,000/10,000-beat conversation. It counts mounted cards, measures
initial headless settling and 30 actual keyboard selection changes, and verifies that selection
changed. The recovery workload queues 100 drafts in a 10,000-passage document, flushes and verifies
the latest snapshot after reopening. These generated sources contain no game dialogue or proprietary
assets.

## Incremental project validation and profiling

The [compiler kernel](../../crates/recite-compiler/src/authoring/state.rs) owns incremental
validation; its differential tests compare accepted results with uncached batch validation. The
Writer probes measure that kernel together with saved-source and search ownership.

```sh
mise exec -- just writer profile cpu 100000 /tmp/writer-cpu
mise exec -- just writer profile heap 10000 /tmp/writer-heap
mise exec -- just writer heap
mise exec -- just writer bench --passages 1000000 --linked --output /tmp/writer-linked.json
```

The profiler builds the existing optimized benchmark with line debug information, then runs the
executable directly under Linux `perf` (DWARF stacks) or DHAT. It records environment/build metadata
and refuses to overwrite a prior profile. DHAT is a development dependency, with its allocator
enabled only in this bench under `heap-profile`; production allocation is unchanged. Instrumented
latencies are not normal-run timings. The executable heap checks own fixed-corpus allocation bounds;
`just writer check` includes them. Investigate an unexplained regression before changing a bound.

## Remaining work and acceptance

Project discovery/indexing and cold validation still scale with corpus size. A high-fan-out semantic
change legitimately invalidates all its dependents; global completeness/recovery changes can still
require a full refresh. Context projection can also be expensive for a single enormous file or many
colliding definitions. Source summaries and saved-search postings remain sizeable heap owners.
Large-scene topology invalidation is still scene-wide, and Source view opens the whole document.

Before claiming RPG production scale, extend these workloads to branching/fan-out, cycles,
cross-scene references, long prose, diagnostics-heavy projects and prolonged editing. Existing
functional tests cover cycles, convergence, conditions, rejected edits, Unicode undo, recovery
failure and conflicts; that is not performance coverage for all those combinations. Measure
end-to-end project discovery/open, scene switching, edit-to-paint, search-to-passage reveal, memory
over hours, crash injection and physical Linux/macOS input/rendering. Screen-reader navigation
across virtual rows and IME composition remain manual acceptance requirements.

Proposed native targets remain 16.7 ms camera/guide frames, p95 edit-to-paint below 50 ms, and warm
search results below 100 ms on named hardware. Current results do not establish those targets.
