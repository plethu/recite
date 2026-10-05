# Bounded LSP latency experiments

Later sustained-load, text-sync, installed-editor and CI experiments are recorded
in the [experience follow-up](follow-up.md).

The verified cancellation and region-cache checkpoint is `6abcb759`. Experiments
below are local Linux stdio measurements, not editor frame timings. No builds,
tests or samplers ran during normal-release timing comparisons. Each edit study
uses three alternating pairs, two warmups and 21 recorded samples per workload
per process. Diagnostic hashes must match the control; valid edits must publish
no diagnostics. Power state is recorded per run.

## A: Move fresh region outputs — rejected

Question: can consuming fresh/relocated region arrays, while borrowing unchanged
ones, reduce newline and cold-index cost without slowing ordinary edits?

This disposable prototype changed only four compiler composition files. It kept
exact-equality and shared-slice reuse, moved owned summary/fact/diagnostic arrays,
and reserved final capacities. The exact patch against `6abcb759` is retained as
`move-output-prototype.patch`; production code was restored after measurement.

| Large workload | Control median | Prototype median |
| --- | ---: | ---: |
| Prose | 3.13 ms | 3.29 ms |
| Newline | 7.10 ms | 6.48 ms |
| Stable-ID label | 4.66 ms | 5.53 ms |
| New block | 14.02 ms | 14.59 ms |
| Recovery transition | 21.06 ms | 21.34 ms |
| Initial index, five alternating process pairs | 385.59 ms | 371.94 ms |

Raw observations are `move-edit-paired.json` and `move-cold-paired.json`. Cold
here means a new server process; filesystem caches remain warm. The first
control startup was 530 ms, so use the medians rather than that outlier.

The compiler suite and Clippy passed; diagnostic hashes matched; both unchanged
writer heap budgets passed. `move-heap.json` retains allocation evidence. Linked
writer peak live allocation fell from 15.70 MB to 14.98 MB. The newline/index
and allocation savings do not justify the roughly 19% stable-ID latency
regression and added ownership plumbing. Keep the current simpler composition
until a more compact representation proves a broader benefit.

## B: Shared text identity equality — retained

The comment profile showed repeated comparisons of unchanged project text in
partition fingerprints, saved-input maps and changed-key detection. On the
pinned Rust 1.96 toolchain, `Arc<str>` equality scans bytes even for the same
allocation: the pointer shortcut depends on `MarkerEq`, whose blanket
implementation is sized. This is visible in the local toolchain's
`alloc/src/sync.rs` (`ArcEqIdent`) and `alloc/src/rc.rs` (`MarkerEq`).

Saved/open compiler inputs and the private LSP fingerprint now explicitly use
`Arc::ptr_eq` before exact text equality. This retains value equality for
independently allocated inputs. Keys and versions still participate; immutable
shared text is sufficient proof only for the text field. No hashes, cache
invalidation rules or retained data were added. The manual compiler equality
implementations destructure every field so adding a field requires updating
the comparison.

All three alternating pairs below ran on battery, balanced platform profile,
with the powersave governor. Compare within this table, not against experiment
A's AC timings. Each cell combines 63 samples from three fresh processes on
the 10.36 MB large project, editing its 518 KB first source file.

| Workload | Control median / p95 | Retained median / p95 |
| --- | ---: | ---: |
| Comment | 6.09 / 7.49 ms | 3.56 / 4.66 ms |
| Prose | 5.83 / 7.51 ms | 3.59 / 4.46 ms |
| Newline | 11.40 / 13.11 ms | 8.93 / 11.24 ms |
| Stable-ID label | 8.32 / 9.97 ms | 6.07 / 7.52 ms |
| New block | 17.56 / 20.76 ms | 17.11 / 19.76 ms |
| Recovery transition | 27.77 / 29.65 ms | 27.02 / 28.89 ms |

`identity-edit-paired.json` retains raw timings, per-run power state, source and
binary hashes, and diagnostic hashes. All diagnostic hashes matched; valid
edits stayed diagnostic-free. The roughly 39% prose and 22% newline reductions
justify keeping this small change. The smaller new-block/recovery differences
are not strong evidence of an improvement.

Separate CPU profiles (`identity-profiles.json`) support the explanation:
fingerprint equality accounted for 14.43% of inclusive comment cycles in the
control, and saved-document equality also appeared prominently. Neither appears
above the 1% report threshold afterward. The estimated total sampled user
cycles for 500 comment edits fell from 7.53 billion to 4.14 billion. Inclusive
shares overlap, and sampled call chains are incomplete; do not add them.

The three-process interactive probes in `identity-interactive-control.json`
and `identity-interactive.json` retained identical request-result hashes across
all eight query/action kinds. All 21 candidate cancellations returned `-32800`
with no edit, with a 0.53 ms median and 1.04 ms maximum. Each three-edit burst
published only version 19. These sequential probes verify behavior and record
timings; the alternating edit study above is the stronger latency comparison.

Validation: compiler/LSP tests, all 1,558 workspace tests (three existing skips),
workspace Clippy with all targets/features, formatting, test organization,
changed-source structural/lint checks, and Git policy passed. Both unchanged
writer heap budgets passed (`identity-heap.json`). The complete repository gate
already passed for the preceding cancellation/region checkpoint; this equality
change was checked through the affected Rust workspace and writer heap lanes.

## Candidates selected for follow-up

After B, comment samples attribute about 18% of inclusive cycles to region
assembly, 14% to stable path identity and 10% to schema loading. These are
investigation priorities, not promised additive savings.

- Reuse a parsed schema when newly read disk bytes are unchanged. Keep the disk
  read and overlay/close/error behavior so edits still observe schema changes
  without depending on a watcher notification. Compare schema-heavy projects
  and test replacement, deletion and overlay closure before retaining it.
- Resolve each distinct path once within an analysis transaction. Keep refresh
  boundaries and alias-retargeting checks; test nested projects and symlinks.
- For newline cost, profile compact span relocation and summary assembly.
  Experiment A demonstrates that merely moving arrays can worsen other edits;
  require wins across prose, stable IDs and structural edits as well.

## C: Reuse unchanged schema parses — retained

Control: `7f64d268`. The saved schema still resolves its configured path and
reads disk on every refresh. Only parsing is reused, after exact bytes, resolved
path, configured path and format match; live overlays are not saved-state cache
entries. This also reuses parse diagnostics for unchanged invalid files while
preserving fresh I/O errors and diagnostic paths after alias retargeting.

Three alternating pairs on battery, 21 samples per workload per process:

| Workload | Standard control / candidate | Schema-heavy control / candidate |
| --- | ---: | ---: |
| Comment | 3.21 / 2.82 ms | 179.13 / 4.57 ms |
| Prose | 3.38 / 2.96 ms | 179.21 / 4.67 ms |
| Newline | 8.39 / 8.49 ms | 185.12 / 10.13 ms |
| Stable-ID label | 5.76 / 5.53 ms | 181.88 / 6.95 ms |
| New block | 18.48 / 16.12 ms | 194.77 / 17.80 ms |
| Recovery | 26.87 / 26.31 ms | 201.88 / 27.99 ms |

Values are medians, not latency budgets. The schema-heavy fixture copies the
large project and replaces its schema with `schema-heavy-fixture.json`, adding
1,000 unused speaker declarations. Its large benefit exposes schema-size
sensitivity that the ordinary fixture misses; it is not a claim about every
project. Raw observations and exact diagnostic hashes are in
`schema-edit-paired.json` and `schema-heavy-paired.json`; all hashes matched.

Validation: the LSP library and integration suite, all-target/all-feature LSP
Clippy, formatting and test organization passed. New differential tests cover
replacement, unchanged invalid input, deletion, recovery, overlay closure and
identical invalid bytes behind a retargeted symlink. `schema_index.rs` remains
cohesive at 258 lines: it owns schema construction and overlay/base selection;
the existing sidecars own diagnostics and lifecycle publication.

## D: Resolve partition paths once per rebuild — rejected

Control: `4c2421c2`. The disposable `PartitionPaths` grouped saved documents by
partition and resolved each open path's partition and retirement target once.
Fingerprinting and request assembly shared those results. Nothing survived a
rebuild, preserving later filesystem refreshes. The exact patch is
`paths-prototype.patch`; production code was restored after measurement.

Three alternating pairs per study, on battery; medians in milliseconds:

| Workload | Large control / candidate | Large repeat | 80-file control / candidate |
| --- | ---: | ---: | ---: |
| Comment | 3.40 / 3.15 | 3.00 / 2.58 | 3.13 / 2.89 |
| Prose | 3.11 / 2.84 | 2.87 / 3.03 | 2.81 / 2.90 |
| Newline | 8.19 / 8.51 | 7.84 / 8.39 | 3.88 / 4.07 |
| Stable-ID label | 5.10 / 5.55 | 5.81 / 5.37 | 3.70 / 3.35 |
| New block | 16.93 / 16.90 | 16.91 / 16.26 | 6.58 / 6.45 |
| Recovery | 26.58 / 26.29 | 26.81 / 26.70 | 9.17 / 9.20 |

Raw data: `paths-edit-paired.json`, `paths-repeat-paired.json` and
`paths-wide-paired.json`. The wide fixture has the same 5,000 blocks in 80 files,
with a 131 KB edited source instead of 518 KB. Exact diagnostic hashes matched
in every pair. The LSP suite, including nested discovery, alias replacement,
retirement and symlink tests, and all-target/all-feature Clippy passed.

Comment gains repeat, but prose gains do not and newline edits consistently
slow by roughly 4–7%. The added allocation/grouping layer does not earn its
complexity. This rejects this implementation, not all filesystem optimization:
schema matching, manifest path checks and snapshot construction still resolve
paths outside this grouped-input layer.

## E: Relocate spans in final assembled output — retained

Control: `4c2421c2` (the intervening D checkpoint contains evidence only).
Previously, every shifted cached region cloned its summary, facts and local
diagnostics into temporary arrays; assembly then cloned those arrays again.
The retained implementation assembles once and shifts each region's ranges in
that final output. It keeps immutable snapshot ownership and the existing exact
reuse comparisons for edits that do not move regions. Checked coordinate failure
discards the candidate and falls back to fresh analysis. Relocation checks
cancellation between regions.

Three alternating pairs per study, on battery; medians in milliseconds:

| Workload | Large control / candidate | Large repeat | 2 MB source control / candidate |
| --- | ---: | ---: | ---: |
| Comment | 2.85 / 3.09 | 3.29 / 3.12 | 8.89 / 8.33 |
| Prose | 2.86 / 2.81 | 3.05 / 3.23 | 8.88 / 8.72 |
| Newline | 8.25 / 5.80 | 8.20 / 6.31 | 28.04 / 18.83 |
| Stable-ID label | 5.59 / 5.17 | 5.45 / 5.58 | 16.69 / 17.06 |
| New block | 17.16 / 17.39 | 16.55 / 16.38 | 57.55 / 58.52 |
| Recovery | 26.66 / 26.66 | 26.95 / 26.73 | 101.24 / 101.85 |

Raw data: `relocation-edit-paired.json`, `relocation-repeat-paired.json` and
`relocation-narrow-paired.json`. The narrow fixture retains 5,000 blocks in five
files; its edited source is 2,072,808 bytes. All paired diagnostic hashes match.
The newline reduction repeats at roughly 23–33%; other workloads have smaller,
mixed changes and do not support a broad speedup claim.

`experiments-profiles.json` records the final CPU samples: 500 comments, 300
newline edits, 200 malformed-indentation edits and 80 recovery transitions.
There were no lost samples in the three symbol reports. Incomplete call chains
and unsymbolized libc frames limit attribution. Path resolution remains visible
in comment work; newline work includes line scanning, cloning and allocation.
Schema loading no longer appears above the 1% report threshold.

The final interactive probe (`experiments-interactive.json`) matches all eight
request/action result hashes from checkpoint B. All 21 cancellations returned
`-32800` without an edit: median 0.51 ms, maximum 0.93 ms. Every burst ended at
version 19; one also published version 17. This harness sends full documents
sequentially and allows an early edit to finish before later edits arrive; the
controlled stdio tests check stale-result fencing explicitly.

The compiler suite and new tests for preserved published snapshots and invalid
cached-coordinate fallback passed. The full `mise exec -- just check` gate
passed for the combined retained changes: 1,563 workspace tests (three existing
skips), writer tests and both unchanged heap budgets, Clippy, editor/engine
integration, documentation and benchmark smoke. `experiments-heap.json` retains
the writer allocation measurements. The working-tree structural scan added no
lint permissions. All touched relocation/composition modules remain below the
production size scrutiny threshold.
