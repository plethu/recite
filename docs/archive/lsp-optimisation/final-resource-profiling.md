# Final LSP CPU and allocation investigation

Historical evidence archived from `docs/design/lsp-cancellation/final-resource-profiling.md` at `58b8f04965af`.
This records its named revision and execution profile; it is not current
workflow or implementation authority. Historical commands use their original
revision and paths. See the [archive index](../README.md).

Investigation date: 7 October 2026. Control revision: `00055c8c`.
This completes the final bounded profiling pass requested after the dependency
spikes. It refreshes CPU attribution, records a Criterion baseline, and measures
real-server allocations. The retained changes only adjust collection capacity;
protocol ownership, cancellation, validation and dependency choices are unchanged.

## Method and evidence

The local host was an AMD Ryzen AI 7 350, Linux 7.2.9-1-cachyos, Rust 1.96.0,
on battery, with the balanced platform profile and powersave governor. These
are diagnostic observations, not a release baseline or cross-platform claim.
All builds completed before uninstrumented timing/CPU comparisons.

Criterion 0.8.2 ran all eight LSP groups for medium, large and realistic:v1-pack,
with ten samples, 500 ms warmup and a one-second requested measurement time.
Slow operations extended collection automatically. This is one local baseline,
not an alternating historical regression comparison. Raw estimates, confidence
intervals and samples are retained with the fixture and build identities.

The real-server large fixture has 20 source files, 5,000 blocks, 50,000 dialogue
lines, 10,000 choices and 10,358,744 source bytes. The edited file is 518,378 bytes.
Seven workloads cover ranged comments, newline insertion, structural edits,
syntax recovery, 100 unknown destinations, 5,000-item completion and a rename
with all local diverts redirected to one block. Inputs remain on disk unchanged.
Diagnostic and query fingerprints must agree across controls and candidates.

For CPU attribution, a release build with line-level debug information, frame
pointers and unwind tables ran 300 operations per workload. `perf` sampled only
the server, after initialization/opening, at 997 Hz with user-cycle events and
DWARF call chains. Every capture reported zero lost samples. Some libc symbols
and call chains remain unresolved; selected self-cycle percentages do not
explain all CPU work or measure wall-time savings. Inclusive shares overlap.

For allocation attribution, an isolated optional DHAT 0.3.3 build ran twelve
operations per workload. A diagnostic monitor records heap counters at acknowledged
markers after opening, after the workload, and after restoring/closing the file.
Initialization and shutdown remain in each complete allocation trace; operation
volume is the counter difference between the first two markers. Small monitor
allocations are included in those deltas. Closing observations include a 300 ms
pause and a definition request; they are not a general leak guarantee.
DHAT timing is excluded from performance comparisons and its allocator metadata
is not an ordinary process-RSS measurement. No profiling dependency or hook is
retained in production.

The [compressed evidence archive](final-resource-evidence.json.gz) retains
Criterion samples, CPU symbol/caller
reports, complete DHAT allocation traces, counter checkpoints, binary/fixture
identities, and uninstrumented comparisons. Raw `perf.data` files are local
investigation artifacts; the readable reports and capture/loss records survive
in the archive. The [diagnostic patch](allocation-probes.patch) and
[capacity patch](capacity-prototype.patch) reproduce the instrumentation and
capacity hypothesis from the named control revision.

## Criterion baseline

Values are mean wall time in milliseconds. The workspace driver bypasses the
coordinator, protocol text store, cancellation scheduling and stdio. Batched
refresh/open cases prepare a new workspace for each measured operation. The
refresh case is its first real comment edit, not a warmed editing session.
The stale-change case also performs an accepted open before the rejected edit;
its total must not be described as stale-version-check latency. Indexing includes
workspace construction, summary counting and teardown with a warm filesystem.

| Group | Medium | Large | Realistic v1 pack |
| --- | ---: | ---: | ---: |
| Initial index | 75.029 | 406.957 | 4.602 |
| Open file | 4.276 | 26.226 | 0.558 |
| Change refresh | 4.880 | 27.753 | 0.633 |
| Diagnostics refresh | 4.709 | 26.440 | 0.652 |
| Completion | 0.135 | 0.561 | 0.0045 |
| Definition | 0.053 | 0.141 | 0.0028 |
| Rename | 0.024 | 0.071 | 0.0038 |
| Stale change workload | 4.828 | 25.708 | 0.617 |

These values describe the control. They do not replace the persistent stdio
workloads or imply that the final server takes 28 ms for every ordinary edit.
The LSP index report estimated 25,388,200 bytes; the direct heap profile below
measures broader live allocations and should not be conflated with that estimate.

## CPU and allocation hotspots

The uninstrumented control ran 96 operations per workload. Process CPU covers
all server threads during the operation sequence, including delivery/destruction;
wire timing measures through client receipt. CPU accounting resolution limits
small differences. Range construction occurs before the send timer.

| Workload | Median wire time, ms | Process CPU per operation, ms | Allocated MiB per operation |
| --- | ---: | ---: | ---: |
| Comment at EOF | 3.495 | 3.542 | 4.011 |
| Newline insertion | 6.137 | 6.146 | 9.947 |
| Structural edit | 16.840 | 16.771 | 18.546 |
| Recovery transition | 25.910 | 25.625 | 35.082 |
| 100 diagnostics | 18.344 | 18.229 | 22.112 |
| 5,000-item completion | 3.735 | 4.896 | 16.494 |
| Local fanout rename | 1.359 | 1.771 | 1.565 |

The profiler identifies these distinct costs:

- EOF range lookup: `text_sync::byte_offset` accounts for 11.36% of sampled
  self cycles in the comment workload. This is an end-of-file case; it does not
  establish the same cost for beginning-of-file prose or full replacement.
- Structural validation: persistent-map lookup accounts for 12.78% of self
  cycles in structural edits and 8.59% in recovery. Lowering, local validation,
  dependency traversal and allocation also remain visible.
- Query responses: completion spends substantial CPU constructing typed items,
  converting them to JSON and encoding the response. Allocation traces expose
  repeated vector growth alongside symbol construction and JSON maps. No custom
  serializer or transport follows from these profiles.
- Startup and composition: AST statement/metadata construction and temporary
  validation sources dominate several cumulative allocation stacks. Region
  summary/fact composition also grows output arrays without an exact size hint.
  Those complete traces include startup; their stack totals are not per-edit
  allocation measurements.

The control's open-file live heap was about 90.1 MiB. After workloads it was
90.1–90.9 MiB, and after restoration/closure about 89.6 MiB. The changed working
state accounts for part of that difference. These bounded checkpoints complement
the existing session growth gates; they do not establish leak freedom.

## Capacity changes

One private `region_outputs::collect` helper sums known slice lengths, reserves
one output vector, and clones slices in their existing order. Summary and fact
composition use it; fact arrays remain boxed slices. Existing shared-output
identity checks and immutable snapshots remain intact. This avoids geometric
output growth without changing the representation or introducing a cache.

An independent two-line completion refinement initializes capacity from the
candidate count and reserves for the known project-symbol count before appending.
The same items, sort/deduplication, cancellation checkpoints and JSON path remain.

Isolated allocation comparisons preserved every diagnostic/query fingerprint:

| Metric | Control | Candidate | Result |
| --- | ---: | ---: | --- |
| Open-file live heap | About 90.1 MiB | About 83.9 MiB | About 6.2 MiB less |
| Allocation through opening | About 834 MB | About 770 MB | About 7.7% less |
| Newline allocation per operation | 9.947 MiB | 6.876 MiB | About 31% less |
| Structural allocation per operation | 18.546 MiB | 15.476 MiB | About 17% less |
| Recovery allocation per operation | 35.082 MiB | 34.074 MiB | About 3% less |
| Diagnostic-heavy allocation per operation | 22.112 MiB | 18.981 MiB | About 14% less |
| Completion allocation per operation | 16.494 MiB | 11.264 MiB | About 32% less |

Region composition does not reduce comment/query operation allocation volume;
it reduces the retained project footprint. Completion reservation is responsible
for the completion-volume reduction. Allocation counts change much less than
bytes because strings and JSON entries still allocate individually.

The final candidate also completed all 24 Criterion cases against this saved
baseline. Large initial-index mean was 391.951 ms and change-refresh mean was
26.250 ms. Other large means remained close to the control; the full statistical
output is retained. This single comparison does not supersede the alternating
real-server comparisons below.

## Uninstrumented comparisons

The normal paired gate passed all 24 workloads in three alternating pairs, with
21 observations after two warmups per workload and matching fingerprints. The
same symbol-enabled build configuration and dependency versions were used on
both sides. No second regression round was required. Peak process RSS decreased
only about 0.5–0.6% (roughly 123,944 to 123,260–123,304 KiB); the 6.2 MiB live-heap
reduction is not a claim of an equivalent resident-memory decrease.

Two small gate results warranted checking: negotiated comments had a median
paired ratio of 1.111, with increases below 0.5 ms, and no-op code actions had a
ratio of 1.457, with increases below 0.7 ms. Longer warmed spot checks kept the
same inputs and budgets: sixteen warmups, 96 comments and 1,024 code actions
per binary in three alternating pairs. Comment ratios were 0.993/1.050/1.005;
code-action ratios were 0.838/1.084/0.961. The increases did not repeat
consistently. This does not assert that every sample stayed within 5%.

A separate three-pair CPU comparison ran 96 operations per workload, reading
process CPU only around each complete operation sequence. The fixed-working-set
requests, diagnostics and fixture hashes matched. Median paired CPU changes:

| Workload | Process CPU change |
| --- | ---: |
| Comment | −5.7% |
| Newline insertion | −8.5% |
| Structural edit | −5.0% |
| Recovery | +0.8% |
| Diagnostic-heavy edit | −1.1% |
| Completion | −4.7% |
| Fanout rename | −5.9% |

Repetition variability and CPU-accounting quantization limit small differences.
Recovery and diagnostic-heavy CPU are effectively unchanged; these results do
not establish a universal CPU or energy reduction. The reliable findings are
lower allocated bytes and live heap without a demonstrated material regression
in the exercised workloads.

## Stopping and future investigations

Retain exact-sized region composition and completion reservation. Their scope
is collection capacity, with one small private helper and two completion lines;
there is no representation, semantic, protocol or dependency migration.

Stop this optimisation pass after the final repository checks. Reopen further
work from a reproduced symptom or a materially different workload:

- If end-of-file ranged conversion dominates a real editing profile, test a
  narrow newline-search replacement against the strict CR/CRLF/LF and UTF-16
  corpus. Do not revive the combined string-library migration on the basis of
  one EOF sample. The dependency record retains its previous mixed results.
- If structural/recovery CPU becomes a practical problem, profile the affected
  dependency traversal before changing persistent indexes or validation ownership.
  Current profiles do not justify a second analysis representation.
- If large response construction dominates real queries, measure item/symbol
  construction and serialization separately. Avoid custom protocol serialization
  or transport for an unproven saving.
- If measured resident memory becomes excessive, distinguish allocator retention
  from live summaries, facts, source and temporary ASTs. The index estimate is not
  a heap measurement, and capacity reductions do not establish an RSS guarantee.

The [dependency reevaluation plan](dependency-decisions.md#reevaluation-plan)
still governs adoption experiments. Release baseline ownership remains with
#109. Platform/session integration evidence predates these capacity changes;
this investigation adds Linux resource evidence, not a new hosted matrix result.

## Reproduction

Start from a disposable export of `00055c8c`. Apply `allocation-probes.patch`
there, then build the plain and heap binaries separately, retaining each binary.
Build flags for both are:

```sh
CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_PROFILE_RELEASE_STRIP=none \
RUSTFLAGS='-C force-frame-pointers=yes -C force-unwind-tables=yes' \
cargo build --locked --release -p recite-lsp
```

Add `--features allocation-profile` for the heap binary. Its profiling environment
is set by `final-profile.py`; do not launch it as an ordinary editor server.
Generate the large project with the exported fixture generator and use the same
fixture path for both sides:

```sh
cargo run --locked --release -p recite-fixturegen -- \
  --profile large --output target/final-large --summaries target/final-large.json
python final-profile.py --binary /path/to/plain-lsp --root target/final-large \
  --output target/final-cpu --mode cpu --samples 300
python final-profile.py --binary /path/to/heap-lsp --root target/final-large \
  --output target/final-heap --mode heap --samples 12
```

Run these from the export root. Apply `capacity-prototype.patch` before building
the final candidate. Applying only its three compiler-file hunks isolates region
composition; the LSP-file hunk adds completion reservation. Import/formatting
cleanup in the retained patch does not change the isolated region hypothesis.
Rebuild both feature configurations before measuring, without concurrent builds
or other measurements. `--mode control --samples 96` records uninstrumented
operation CPU. The archived scripts provide alternating comparisons:

```sh
python final-cpu-pairs.py --probe final-profile.py \
  --control /path/to/control-lsp --candidate /path/to/candidate-lsp \
  --project target/final-large --output target/final-cpu-pairs.json
python final-spot-check.py \
  --control /path/to/control-lsp --candidate /path/to/candidate-lsp \
  --project target/final-large --output target/final-spot-check.json
```

The ordinary `scripts/check-lsp-performance.py` accepts those two plain binaries
and the generated project. Criterion reproduces with:

```sh
RECITE_BENCH_SCALES=medium,large,realistic:v1-pack \
cargo bench --locked -p recite-benchmarks --bench lsp -- --noplot
```

See the evidence archive for exact sample counts, fixture hashes, build flags,
versions and binary identities. Reproducing on another host need not reproduce
these timings or binary hashes.

## Final verification

All complete repository verification lanes passed across the initial run and
resumed lanes: 1,573 workspace tests, three existing skips, all 48 CI contract
tests, Clippy, both unchanged Writer heap budgets, editor/engine checks,
dependency policy, documentation and benchmark smoke. Scratch space had to be
outside the repository and on disk; dependency policy also needed access to its
Cargo advisory cache. Those environment corrections did not change any gate.

The archived reproductions applied successfully to a fresh export of `00055c8c`
and produced all four retained production files byte for byte; all three Python
probes parsed. Local document links, anchors, compressed JSON and patch checks
passed. Touched production modules remain below the 250-line scrutiny threshold.
The spelling dictionary now recognizes the published `writeable` crate identifier
in older archived lockfile evidence without changing that dependency name.
