# Writer scalability

The writer now bounds much of its visible work, shares saved source text, and
indexes project search. It is **not yet ready for comfortable million-passage
editing**: a committed edit in the generated million-passage project still takes
about 2.4 seconds. The remaining project-wide semantic validation and memory
working set need another compiler/kernel pass. Freya remains the selected UI.

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
edits, ten undos and ten redos. It reports source bytes, estimated retained history
allocation and Linux process peak RSS. Opening here excludes filesystem discovery,
index construction and painting: those are not one end-to-end project-open timing.

The GUI workload is a separate 1,000/10,000-beat conversation. It counts mounted
cards, measures initial headless settling and 30 actual keyboard selection changes,
and verifies that selection changed. The recovery workload queues 100 drafts in a
10,000-passage document, flushes and verifies the latest snapshot after reopening.
These generated sources contain no game dialogue or proprietary assets.

## Local results, 16 September 2026

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

## Remaining work and acceptance

The next performance frontier is dependency-aware semantic validation in the
shared authoring kernel, with correctness tests for cross-file references, schema
changes, ID collisions and diagnostics invalidation. Moving validation off-thread
alone would hide a stall while retaining its cost; measure allocation ownership
and retain only the document analyses needed by the accepted project snapshot.
Large-scene topology invalidation after source/placement changes also remains
scene-wide. Source view still opens the whole document in the existing code editor.

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

## Implementation review

Projection caches and history belong to the document model; disk ownership and
job lifecycle belong to the frontend. Neither changes runtime traversal or the
public saved-document constructor/accessors. Recovery still uses the existing
serialized format. File buffers, project errors, reading context, script paging
and cached graph geometry now have separate owners. The larger `lib.rs` remains
the application composition/root-state owner; `scene_map.rs` composes the map
surface while layout, routing, culling and neighbourhood selection live in
focused modules. Further splitting their remaining composition code would mostly
move hook wiring rather than establish a new responsibility.
