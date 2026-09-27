# Writer scalability

The writer bounds much of its visible work, shares saved source text, and indexes
project search. The authoring kernel retains compact validation facts and updates
only changed documents and affected dependents. Single-line, multiline and ID edits
now avoid project-wide validation in the generated million-passage workload.
Repeatable workloads and heap checks below cover model latency, allocation, and
regression limits. Freya remains the selected UI; model measurements do not
establish native GUI readiness.

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
