# LSP dependency decisions

Assessment date: 6 October 2026. Recite base: `a524d281`.

This is the decision record for LSP dependency adoption. It preserves both
successful controls and negative results; the patches and raw measurements below
are disposable experiment evidence, not a second maintained implementation.
The [reevaluation plan](#reevaluation-plan) identifies when to reopen each decision.

## Adoption job and acceptance

Reduce the LSP code Recite must maintain without weakening ordered edit
acceptance, atomic batches, UTF-16 validation, bounded work, snapshot freshness,
cancellation through final writer handoff, or shutdown. A roughly 5% slowdown
can be acceptable for substantial net maintenance reduction. Count adapters,
runtime setup, retained representations and validation against gross deletions.

The primary session and Astra independently inspected the ownership boundaries
and agreed the decisions below after bounded released-crate probes. No dependency
or production code changes are retained. No upstream publication was performed.

## Decisions

| Candidate | Decision | Evidence and reopening condition |
| --- | --- | --- |
| `lsp-server` 0.7.9 | Keep | Already owns framing and stdio transport. Its rust-analyzer home has continuing maintenance. Revisit for a reproduced component defect or concrete missing capability. |
| `gen-lsp-types` 0.11.0 | Preferred future types replacement; defer this migration | Maintained generated protocol coverage and real rust-analyzer/Ruff adoption are positive. Current edits require a strict ingress representation; no needed new editor capability or net maintenance reduction was demonstrated. Revisit for an upstream edit-union repair, a needed protocol feature, or a dedicated protocol modernization task. |
| `tower-lsp-server` 0.24.0-rc.1 | Defer framework integration | Async notification handlers may overlap; concurrency one disables effective cancellation. Cancellation ends at handler completion, before writer delivery. Preserving Recite's stronger rules retains its coordinator and adds integration machinery. Revisit only with a supported integration boundary that removes material ownership. |
| `async-lsp` 0.2.4 | Defer | Released main-loop/concurrency composition stalls at saturation even after pending work becomes ready. A capacity-two control completes. Revisit after a released repair and a credible deletion inventory; do not assume ownership of a fork. |
| `lsp-textdocument` 0.5.0 | Reject for this edit boundary | Surrogate-interior positions round down; a failed batch can mutate earlier edits before panicking. Recite must retain strict validation, transaction staging, stale-version checks and snapshot normalization. This replaces little of the owned complexity. Revisit if an API directly supplies those contracts. |
| Salsa | Defer | Reputable incremental-analysis dependency, but it changes semantic analysis and invalidation ownership. No bounded query or maintenance bottleneck currently earns that migration. |

These are fit decisions, not claims that these projects are generally low quality.
The generated-types crate remains the strongest future adoption candidate. Its
adapter is affordable: the probe's three-field strict representation preserves
old deserialization and retains `rangeLength`. That does not remove Recite's
contextual validation or establish whole-server wire parity.

Our current `lsp-types` 0.97.0 upstream last committed in June 2024; regard it as
frozen rather than an actively maintained long-term choice. Avoid original
`tower-lsp` for a new migration. Avoid `ls-types`, archived in August 2026 in
favour of generated types. The Tower community fork has real recent fixes and
cross-platform CI, but its maintainers explicitly discuss limited capacity;
Tower's generated-types integration is in 0.24.0-rc.1, a release candidate;
`gen-lsp-types` 0.11.0 itself is not that release candidate.

## What a framework could actually delete

The current server already delegates framing. At the base above, potential
replacement sites are request decoding (`query.rs`, 40 lines), notification
decoding (`updates.rs`, 25), initialization handshake (5), stdio startup/join
(6), and message routing (8): **84 gross lines before replacement glue**.
This is an inventory, not a completed migration or promised net saving.

Shutdown interruption/draining, pending request states, final-handoff
cancellation, freshness fences, coalescing barriers, capacity policy, publication
pruning and worker isolation remain Recite-owned. Counting whole server modules
as removable framework boilerplate would substantially overstate the benefit.

## Bounded experiments

The [disposable probe patch](dependency-probes.patch) contains a standalone
Cargo package, exact candidate versions, its resolved lockfile and source.
It is evidence, not a second maintained server or CI dependency lane.
The [recorded output](dependency-results.txt) contains the observed results.

1. Generated edit unions: valid full/ranged controls; incomplete/wrong-type
   ranges; negative positions; malformed/negative lengths; a full replacement
   with a non-null length. Malformed partial changes fall back to the whole
   document variant, silently dropping validation information. The strict
   representation matches old decoding and retains length information.
2. Initialization: omitted/null/empty `workspaceFolders`. Generated types
   preserve explicit null, whereas old types erase it on serialization. This is
   a genuine protocol-model improvement, not an observed Recite editor fix.
3. Text-document utility: surrogate-interior conversion and a valid replacement
   followed by a reversed range. The latter panics with changed text but the
   old version, proving that this API alone is not our atomic boundary.
4. Tower notification order: suspend the first custom notification explicitly;
   a second notification and barrier request finish before it resumes. This
   proves asynchronous handlers can overlap, not that non-yielding handlers
   necessarily reorder. Concurrency one leaves a pending request uncancelled
   during the 250 ms observation window, as its documentation predicts.
5. Tower blocked output: finish a request, confirm the writer is blocked,
   deliver cancellation followed by an observed barrier handler, then release
   the writer. The successful original response is delivered. The writer already
   owns that response, so this does not establish divergence from Recite's
   rendezvous boundary and is not asserted to violate LSP. Source inspection
   separately establishes Tower's handler-completion cancellation boundary.
6. Async-lsp saturation: two custom requests await an explicit gate. Observe
   `poll_ready` returning pending at capacity one before releasing the gate.
   Its started future does not complete; capacity two completes both requests.
   No repeated shutdown requests or lifecycle-invalid client traffic are used
   to claim a lifecycle bug: this isolates main-loop/concurrency composition.

Timeouts bound probes; they are not latency benchmarks. Candidates stopped at
contract or maintenance-value gates, so no comparative performance improvement
is claimed. A future candidate that passes those gates still needs alternating
paired measurements, semantic hashes and the full editor/session contract.

Reproduce outside the working tree:

```sh
mkdir /tmp/recite-dependency-probes
cd /tmp/recite-dependency-probes
git apply /path/to/recite/docs/design/lsp-cancellation/dependency-probes.patch
cargo run --locked --release
```

The text-document panic is intentionally caught and inspected; its panic hook
can print to stderr even when the probe exits successfully. Unexpected outcomes
fail assertions. Three release repetitions exited successfully with identical
stdout; applying the recorded patch in a fresh directory also reproduced the
result. These probes ran on Linux; they do not establish alternative framework
performance or behaviour on macOS/Windows.

## Retained verification

The stdio incremental-sync integration test now sends malformed raw wire
changes after a valid first edit in the same batch. It checks that accepted
text remains queryable and that a valid edit at the rejected version is still
accepted. Unknown extension fields remain accepted. This protects the boundary
where a future union-type migration could otherwise bypass semantic validation.

`cargo test --locked -p recite-lsp` passed 197 tests including the doctest.
LSP Clippy (all targets/features, warnings denied), formatting, spelling, test
organization and Git policy checks passed. These scoped checks are appropriate
for a test-only production diff and design evidence; the complete workspace
gate was not repeated.

## Additional text and framework spikes

The earlier shortlist was not an exhaustive search for reusable LSP components.
This extension tested published `str_indices` 0.4.4, `line-index` 0.1.2,
Ropey 1.6.1, crop 0.4.3, and **lspf 1.0.3**. The indexed lspf 0.2.0 result
was stale and that release is yanked; 1.0.3 requires Rust 1.98. An isolated
1.98 toolchain ran the probe without changing Recite's pinned 1.96 toolchain.

| Candidate | Experiment and result | Decision |
| --- | --- | --- |
| `str_indices` | Strict adapter matched all 124,800 position cases. A real LSP prototype replaced three conversion loops and the edit offset scan, deleting 15 net production lines. Six balanced comparisons showed useful ranged gains but mixed overall timing. | Archive this prototype. Preferred future utility for a measured counting/conversion hotspot; a narrower count-only migration has not been tested. |
| `line-index` | Cached lookups were fast. The LF/CRLF adapter matched its subset of the corpus; bare-CR handling differed. Building an index for the 780 KB microbenchmark document took about 1.27 ms, so cached lookup alone is not an integration result. | Defer for LF-based compiler/query indexing. Not a direct replacement for protocol edit coordinates without additional compatibility ownership. |
| Ropey | Explicit `cr_lines`/`simd`, with defaults disabled, matched the entire corpus. A real private protocol-store prototype passed the LSP suite and deleted 21 net production lines. Indexed editing improved isolated middle/end edits, but end-to-end full replacements regressed. | Archive this prototype. Revisit if transaction copying/offset lookup dominates real editing, or analysis can consume chunks without flattening. Do not expand analysis architecture merely to make this adoption win. |
| crop | With an explicit surrogate round-trip check, its LF/CRLF adapter matched its corpus subset. Bare-CR handling differed. Its transaction microbenchmark included flattening and showed real middle/end benefits. | Reject this direct protocol-store adapter. Additional CR indexing/normalization adds ownership; Ropey already fits that contract more directly. This does not reject crop for other text-buffer jobs. |
| lspf | Built-server positive controls passed: atomic failed batch, surrogate rejection, retained snapshots, serial awaited hooks and saturated cancellation. It also accepted stale/equal versions, ignored mismatched `rangeLength`, overwrote duplicate opens, accepted malformed ranges as full replacements, and rejected an overlong column instead of clamping. | Defer despite meaningful framework capabilities. Hooks/Layers run after built-in mutation; a pre-validating transport retains strict transaction ownership. No supported final-publication boundary was found that would remove our coordinator cleanly. Youth and MSRV alone are not rejection reasons. |

### Evidence and scope

The [standalone source and locked manifests](text-spikes.patch) reproduce the
position corpus, transaction microbenchmarks, and built lspf server.
The [str_indices prototype](str-text-prototype.patch) and
[Ropey prototype](rope-text-prototype.patch) apply separately to the current
production source. They are disposable evidence; no runtime dependency is added
on the working branch. The existing raw-wire regression test was included in
both temporary checkouts. Each prototype passed 197 LSP tests and Clippy with
all targets/features and warnings denied.

The corpus covers 1,300 short texts and 124,800 positions, including mixed
CR/CRLF/LF, Unicode, surrogate interiors, nonexistent lines, EOF and overlong
columns. NEL, U+2028 and U+2029 remain ordinary characters. Ropey's feature
configuration is part of correctness: Cargo feature unification could otherwise
turn those into line breaks. The separate lspf package intentionally retains its
own default Ropey features; the text-spike package does not unify with it.

Transaction microbenchmarks use 3.9 KB, 78 KB and 780 KB documents, at beginning,
middle and end positions. They include locating the edit, staging a copy,
flattening, and dropping temporary state. For the 780 KB middle/end cases,
Ropey took about 30 microseconds against String's 217/421; at the beginning it
was slower, about 28 against 16. These independent transactions do not measure
initial rope construction, accumulated chunk evolution or the parser.

The [raw comparison](text-comparison.json) and
[summary/corpus/framework results](text-spikes-results.json) record six balanced
run permutations for each of the medium and large projects. Each run includes
21 samples plus warmups for six edit classes in full and ranged modes and eight
query/action classes. The medium and large edited files are 207,450 and 518,378
bytes, respectively; the microbenchmark's 780 KB document is a different fixture.
All diagnostics and query fingerprints matched. Other existing dependencies
were unchanged in the prototype lockfiles. No compilation or tests ran during
the final comparisons.

Selected large-project medians, in milliseconds:

| Workload | Control | str_indices | Ropey |
| --- | ---: | ---: | ---: |
| Full comment edit | 3.096 | 3.354 | 3.666 |
| Full prose edit | 3.515 | 3.385 | 4.062 |
| Full line insertion | 6.449 | 6.634 | 6.908 |
| Ranged comment edit | 3.545 | 3.234 | 3.363 |
| Ranged block topology | 17.239 | 15.982 | 15.794 |
| Completion | 3.543 | 3.561 | 3.761 |

Paired median ratios differ slightly from ratios of the pooled medians above.
Ropey's full comment/prose paired regressions were about 20.6%; the corresponding
ranged comment/topology gains were about 5.1%/8.1%. str_indices had mixed results,
including a 9.5% paired full-comment regression. Do not interpret tiny query
changes as decisive from percentages alone. The end-to-end measurements establish
the tradeoff, not complete attribution of its cause.

Neither prototype demonstrated a broad enough benefit for its modest maintenance
reduction under the approximately 5% allowance. The primary session and Astra
agree to retain current production. Memory, long-session resources and hosted
macOS/Windows behaviour were not measured for these alternatives; no adoption
or cross-platform performance claim rests on these Linux spikes.

Reproduce the standalone experiments in an empty directory:

```sh
git apply /path/to/recite/docs/design/lsp-cancellation/text-spikes.patch
cargo +1.96.0 run --locked --release --manifest-path text/Cargo.toml
cargo +1.98.0 run --locked --release --manifest-path lspf/Cargo.toml
```

## Reevaluation plan

At the v1 dependency review, refresh release, maintainer and advisory information
for current dependencies and `gen-lsp-types`. Repeat that lightweight review when
planning a protocol upgrade or encountering a dependency defect. A newer version
or an elapsed date alone does not justify repeating the entire experiment suite.
There is no scheduled benchmark job or automatic migration attached to this plan.

| Candidate | When to reopen | First experiment and evidence needed |
| --- | --- | --- |
| `gen-lsp-types` | First priority at a needed protocol-feature upgrade, a released edit-union repair, or a dedicated replacement of frozen `lsp-types`. Check its status during the v1 dependency review. | Rerun malformed-edit decoding and missing/null controls. Preserve strict acceptance through a small ingress representation if necessary; then establish whole-server wire parity. The existing compile-error count is not a rejection criterion. |
| `str_indices` | A profile shows scalar/UTF-16 counting or conversion materially contributes to diagnostics/query latency, or those loops become a demonstrated maintenance problem. | Try a narrow count/conversion replacement before reviving the combined prototype. Measure the affected operation end to end; this narrower variant is currently untested. |
| Ropey | Real editing profiles show transaction copying or offset lookup dominates, representative documents grow beyond the tested sizes, or an independently justified analysis change can consume chunks. | Reapply the protocol-store prototype. Include initial construction, repeated edits, full replacements, flattening, peak memory and Unicode-feature unification. Improve the relevant workload without hiding the previous full-replacement regressions. |
| `line-index` | Repeated LF-based compiler/query conversions become hot and a snapshot can reuse one index across enough operations to amortize construction. | Compare the complete projection path, including construction and retained memory. Preserve scalar columns, source spans and CRLF treatment. Protocol bare-CR support needs its own explicit solution. |
| crop | A released API supports the required CR/CRLF/LF semantics, or a separate chunk-based text-buffer requirement makes its byte-oriented representation useful. | Rerun the complete position corpus, strict surrogate checks and atomic transactions; compare against Ropey as well as String, including flattening. Avoid maintaining a second line index merely to adapt it. |
| lspf | A release offers pre-mutation validation/rejection or custom document ownership, plus a supported publication boundary that can preserve freshness and cancellation. Alternatively, a demonstrated coordinator maintenance burden justifies testing its existing interception APIs. | Rerun the built-server controls and all rejected-input cases. Count the transport interceptor, validation state, encoding adaptation and publication coordination against gross deletions. Record the required toolchain upgrade; a 1.x label alone does not establish fit. |
| Tower community fork | A supported integration hook preserves ordered mutation and the required publication authority, or lifecycle/routing code grows into a material maintenance burden. | Test suspended notification ordering, saturated cancellation and cancellation before writer acceptance. Inventory actual removed owners; a stable release alone does not establish savings. |
| `async-lsp` | A released version repairs saturation, and there is a concrete reason to replace protocol orchestration. | Rerun the explicit readiness/gate probe with its capacity-two control before any backend migration. Then test ordering, shutdown and final publication authority. |
| `lsp-textdocument` | A released transactional/checked API replaces substantial strict edit validation rather than requiring it alongside the dependency. | Replay malformed batches, stale versions, surrogate interiors, `rangeLength` and all newline conventions. Count remaining validation and staging code before benchmarking. |
| Salsa | Profiling or concrete maintenance changes show manual query invalidation/recomputation is a bottleneck; evidence points beyond protocol transport. | Replace one bounded analysis query and compare invalidation ownership, deterministic diagnostics, cancellation, recovery and retained memory. Do not begin with a whole-analysis rewrite. |

For every reopened candidate, refresh maintenance, licensing, supported features
and platform requirements before compiling it. Use the retained reproductions
as a starting point and the then-current production revision as the new control.
Accept a dependency for a substantial net reduction in owned complexity or a
consistent relevant performance/capability gain. A roughly 5% slowdown is a
possible maintenance trade, not permission to weaken correctness or ignore a
supported workload. Name any unmeasured surfaces, including long-session
resources and macOS/Windows, before making adoption claims.

Update this record when a decision changes. Keep old measurements tied to their
versions and execution profile; record the new revision and superseding evidence
instead of silently treating the October 2026 results as current.

## Maintainer tooling language

Reassessed 7 October 2026 after the maintainer challenged the extra language and
learning cost. Retain Python for the external LSP measurement harness; this does
not make it the default for new automation. Rust and Node already have project
toolchains, so extending Python must earn its additional setup and maintenance.

The [bounded spike evidence](tooling-language-evidence.json.gz) preserves source,
an isolated Cargo lock and Linux observations. A cached released `sysinfo` 0.38.4
sampler matched psutil's CPU, RSS, thread and descriptor counters for a child
deliberately growing memory, threads and handles. A Node client using the already
resolved `vscode-jsonrpc` 8.2.0 matched Python on initialization, edit diagnostics
and definition. These prove viable components, not a complete port or a timing win.

| Option | Fit and remaining cost |
| --- | --- |
| Python + psutil | Supplies the current CPU/RSS/thread/descriptor-or-handle contract on the three target platforms. Keep ordinary modules, one CLI, a locked environment and checks; count the extra language and learning cost explicitly. |
| Rust + sysinfo | Linux sampler works. The current `tasks()` API remains Linux-only; preserving other-platform thread checks needs additional dependencies/adapters and verification. The safe libproc task API is a credible macOS extension, not a tested replacement. Use a private maintainer crate if revisiting; `recite-benchmarks` is also a shipped CLI dependency. |
| Node/TypeScript | Protocol and report code can use the existing ecosystem. Inspected pidusage/systeminformation APIs do not supply all thread/handle counters, so moving the full harness still requires another platform-accounting solution. Splitting orchestration from a Python sampler adds a runtime/IPC boundary. |

The decisive benefit today is delegating cross-platform process inspection to
psutil while keeping one measurement owner. Do not reproduce its OS handling
just to remove Python. Conversely, do not keep Python merely because it is
already written. Reevaluate for a small maintained adapter covering all required
counters, or measured driver interference that would materially improve evidence
with another implementation. Preserve gates rather than silently dropping a metric.

Any port must compare against the same server and preserve or explicitly
recalibrate timing definitions. Our reader timestamps a complete body before JSON
decoding; typical protocol-library callbacks happen after decoding. Serialization
also precedes the send timer. No macOS/Windows alternative sampler or full
alternative harness was executed in this assessment.

References: [psutil process API](https://psutil.io/api/),
[sysinfo tasks contract](https://docs.rs/sysinfo/latest/sysinfo/struct.Process.html#method.tasks),
[libproc task inspection](https://docs.rs/libproc/0.14.11/libproc/proc_pid/fn.pidinfo.html),
[pidusage contract](https://github.com/soyuka/pidusage),
[systeminformation process API](https://systeminformation.io/processes.html).

## Primary maintenance and contract sources

- [Generated types and generator](https://github.com/ribru17/gen-lsp-types),
  [rust-analyzer manifest](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/Cargo.toml),
  [Ruff manifest](https://github.com/astral-sh/ruff/blob/main/Cargo.toml).
- [Tower maintenance and types transition](https://github.com/tower-lsp-community/tower-lsp-server/pull/76),
  [notification-order discussion](https://github.com/tower-lsp-community/tower-lsp-server/issues/36),
  [transport](https://github.com/tower-lsp-community/tower-lsp-server/blob/a6ab04da7e6301a5b14a668ae303badc1b574e0c/src/transport.rs).
- [Async-lsp saturation repair](https://github.com/oxalica/async-lsp/pull/30),
  [lifecycle repair](https://github.com/oxalica/async-lsp/pull/29).
- [Text-document source](https://github.com/GiveMe-A-Name/lsp-textdocument/blob/7aa7fadfcd8d48197c85e2dcdc0cf92fd8efaaee/src/text_document.rs),
  [archived types replacement](https://github.com/tower-lsp-community/ls-types).

- [str_indices](https://github.com/cessen/str_indices),
  [line-index](https://docs.rs/line-index/0.1.2/line_index/),
  [Ropey](https://github.com/cessen/ropey), [crop](https://github.com/noib3/crop).
- [lspf published package](https://crates.io/crates/lspf/1.0.3),
  [released document implementation](https://github.com/meymchen/lspf/blob/v1.0.3/crates/lspf/src/documents.rs).
