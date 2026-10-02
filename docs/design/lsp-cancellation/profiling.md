# LSP review follow-up profiling

Local Linux evidence for the cancellation implementation. These experiments
address the adversarial review and identify remaining costs; they do not
establish a cross-platform release latency guarantee. The latest implementation
and measurements are in [Region reuse follow-up](#region-reuse-follow-up).

## Final stdio measurements

The same retained probe uses three fresh release-server processes, two warmups
per request class and seven recorded samples per process. The medium and large
`*-optimized.json` reports preserve raw observations and source/binary/script
hashes. All successful query hashes match the baseline; all 21 cancellations per
scale return `-32800` without edits. Only the final burst version is published.
The normal release build is distinct from the profiling build below.

| Operation | Pre-review large median | Final large median | Final medium median |
| --- | ---: | ---: | ---: |
| Full-text edit through diagnostics | 20.67 ms | 17.86 ms | 6.98 ms |
| Ordinary rename, three edits | 6.04 ms | 0.30 ms | 0.17 ms |
| Open first source | 14.16 ms | 6.44 ms | 3.38 ms |
| Cancellation reply behind an edit | 0.56 ms | 0.57 ms | 0.29 ms |
| Code action with no repairs | 18.80 ms | 18.45 ms | 2.40 ms |
| Fix all, one missing stable-ID suffix | 24.74 ms | 19.13 ms | 2.56 ms |
| Completion | 10.40 ms | 11.75 ms | 2.23 ms |
| Three-edit burst plus completion | 32.67 ms | 33.43 ms | 11.97 ms |
| Initial index readiness | 479.77 ms | 469.24 ms | 79.37 ms |
| Maximum observed process RSS | 169.17 MiB | 150.41 MiB | 45.93 MiB |

Large edit p95 is 26.25 ms; rename p95 is 0.46 ms; cancellation p95 is 0.75 ms.
These are small local sample sets, not release tail guarantees. Startup ranged
from 462.2 to 619.5 ms. Completion and burst did not improve in the retained run;
completion still projects and transports approximately 320 KB of JSON. Battery,
balanced platform profile and powersave governor matched the original setup.
Builds and tests did not run during the retained final measurements.

The dedicated `large-fanout.json` probe rewrites local references in the first
large shard using overlays. It validates every returned range, replacement and
document version. Across three fresh processes and 21 samples per case, medians
were 0.38 ms / 0.86 ms / 2.74 ms / 2.90 ms for 3 / 101 / 501 / 623 edits.
The review's 623-edit observation was approximately 205 ms across three runs,
so this is an indicative comparison with fewer pre-fix observations. The final
623-edit maximum was 4.22 ms. No on-disk fixture was changed.

An additional alternating comparison retained the pre-review profiling binary
and the final profiling binary, both built with the same release/debug/frame
options. Three pairs of fresh servers ran 21 samples per edit kind after two
warmups; no CPU sampler or other build/test ran during this comparison. Pair
order alternated to reduce ordering bias. `edit-paired.json` records binary
hashes and samples. Pooled medians were:

| Edit kind | Before | After |
| --- | ---: | ---: |
| Comment | 19.56 ms | 17.39 ms |
| Prose | 18.75 ms | 17.67 ms |
| Stable-ID label | 18.85 ms | 17.63 ms |
| New block | 29.22 ms | 28.05 ms |

## Changes driven by the profile

The pre-review profile attributed 45.9% of rename's sampled user CPU cycles to
generic symbol construction, with overlapping navigation, references and
collision callers. Cursor lookup now filters borrowed spans before constructing
symbols, preserving the existing full ordering for overlapping hits. A fixture
test compares the selected symbol against full-list lookup at every position.
Collision checks scan block declarations without cloning the project symbol set
and retain the existing completeness requirements and local-collision precedence.

After those changes, an intermediate profile exposed repeated saved-document URI
resolution (40.8%) and source fingerprinting (24.9%). The immutable query index
now has a reverse `(partition, document key)` mapping. Compiler analyses cache
their source fingerprint, which edit preconditions compare without rehashing.
Exact source-to-URI validation still runs once per precondition document.

High-fanout rename previously rescanned source prefixes for every edit endpoint.
An immutable source-bound line index now supports exact scalar byte offsets and
UTF-16 projection. Validation and projection check cancellation between edits.
Line-index tests compare empty, CRLF, Unicode and malformed boundaries against
the previous byte-position helper; projection checks include non-BMP characters.

For edits, the pre-review profile attributed 8.2% to rebuilding all LSP file
summaries and 6.6% to cloning discovery reports. Reports now share immutable
storage. A summary is reused only when its partition/key, URI identity,
metadata, compiler summary and diagnostics still match. That intermediate profile
put LSP snapshot rebuilding at 1.0%, with discovery cloning below the 0.5%
reporting threshold. Changed-file analysis accounted for 70.1%, including
lowering at 27.0%, authoring summary construction at 11.8%, local validation at
10.9% and parsing at 9.4%.

These are **inclusive sampled cycle shares**, not wall-time shares; they overlap
and must not be added. The profiles sampled only the server after indexing and
opening the document: 400 repeated renames, then 150 sequential full-text edits.
`perf record -e cycles:u -F 997 --call-graph dwarf,16384` attached to the owned
server process. A separate optimized build enabled debug information, unwind
tables and frame pointers. No lost samples were reported; some call chains
remain incomplete. Normal release stdio measurements are recorded separately.

The final profile used 3,000 renames and 150 edits. Saved-URI resolution fell
to 1.1% of rename cycles and source fingerprinting dropped below the 0.5%
reporting threshold. Cursor symbol scanning is now the largest rename component
(32.1%), at a much smaller absolute duration. Instrumented medians were 0.13 ms
rename and 17.01 ms edit; these isolated repeated-operation timings are not the
mixed-workflow release measurements above.

Changed-file analysis remained 63.9% of final edit cycles: lowering 31.0%,
parsing 11.5%, summary construction 8.8%, and local validation 6.3%. LSP summary
rebuilding was 0.8%; line-index construction 1.0%; cached fingerprint construction
0.6%. Inclusive shares overlap. Neither final profile lost samples. Raw local
profiles and readable reports are under `/tmp/recite-final-symbolized-*.data`
and `/tmp/recite-final-*-report.txt`.

## Changed-file and project scaling

The generated workloads use seed 7203. `scaling-profiles.toml` retains their
fixture definitions. The intermediate `edit-scaling.json` run precedes the final
URI/fingerprint query optimizations and records source hashes, binary/script
hashes and 21 samples per workload after two warmups. Each project uses one
fresh server. Timings include a full-text update through receipt of that
version's diagnostics. Inputs are editor overlays; disk hashes are rechecked.

| Project / source layout | Edited bytes | Comment ms | Prose ms | Stable-ID label ms | New block ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5,000 blocks / 5 files | 2,072,808 | 70.64 | 72.90 | 72.18 | 127.28 |
| 5,000 blocks / 20 files | 518,378 | 19.78 | 19.92 | 20.14 | 32.20 |
| 5,000 blocks / 80 files | 131,018 | 7.59 | 7.74 | 7.63 | 10.87 |
| 2,500 blocks / 10 files | 518,128 | 18.36 | 19.01 | 18.35 | 29.63 |
| 10,000 blocks / 40 files | 518,628 | 21.06 | 20.80 | 20.64 | 36.02 |

These are medians from one local run, not statistically established tail bounds.
Repartitioning regenerates the project, so source hashes and reference placement
change; the experiment holds generated content scale approximately constant,
not byte-for-byte identity. The last two rows keep the edited shard near 518 KB
while changing total project size fourfold. The result and CPU profile both point
to changed-file analysis as the next major edit-time target.

That profile motivated the region-reuse follow-up described below. The retained
scaling table above measures the earlier implementation; it is not the result of
the region cache.

Reproduce the scaling workload with the standard fixture generator and:

```sh
target/release/recite-fixturegen \
  --profiles docs/design/lsp-cancellation/scaling-profiles.toml \
  --profile wide --output /tmp/recite-scaling/wide \
  --summaries /tmp/recite-scaling/wide.json
python3 scripts/measure-lsp-edit-workloads.py /tmp/recite-scaling/wide \
  --output /tmp/recite-edit-scaling.json
python3 scripts/measure-lsp-rename-fanout.py \
  target/recite-benchmarks/generated/large --output /tmp/recite-fanout.json
```

## Region reuse follow-up

The changed-file opportunity is now implemented. Parser-owned top-level regions
reuse compact summaries, project facts and local diagnostics. Byte-identical
shifted regions relocate their positions. File-wide recovery participation still
controls local validation; semantic project changes still use the existing
validator. The architecture and fallback rules are in
[the cancellation design](../../lsp-cancellation-design.md#reusing-analysis-within-an-edited-file).

### Controlled edit comparison

The machine changed from battery to AC during this work. The before binary was
preserved and its SHA-256 matches the earlier `large-optimized.json` report.
Both binaries were therefore measured again on AC, with the same balanced
platform profile and powersave governor. Comparing directly with the earlier
17.86 ms battery observation would mix code and power-condition changes.

`region-edit-paired.json` records three alternating pairs of fresh normal-release
servers, two warmups and 21 samples per edit kind per process: 63 observations per
cell below. Every run records power state before and after. No build, test or CPU
sampler ran during these comparisons. All per-version diagnostic hashes match
between binaries; valid edit workloads assert that diagnostics remain empty.

| Large-project edit | Before median | After median |
| --- | ---: | ---: |
| Comment | 13.20 ms | 3.06 ms |
| Prose | 13.15 ms | 2.98 ms |
| Insert one/two prose lines | 21.63 ms | 6.89 ms |
| Stable-ID label | 13.32 ms | 4.92 ms |
| Append a block | 21.72 ms | 14.08 ms |
| Enter/leave syntax recovery | 126.28 ms | 21.36 ms |

The newline probe preserves the original indentation and alternates one/two
added lines, forcing suffix relocation on successive samples. An early probe
used the wrong indentation in a nested passage and unintentionally exercised
recovery. It was corrected before these retained comparisons. The recovery case
explicitly alternates a stray top-level token and a valid comment.

The recovery profile led to a further dependency correction: a project-wide
stable-ID completeness transition only affects choice-echo lookup. Revalidating
unrelated documents for that transition was unnecessary. Echo consumers still
revalidate in both directions, along with ordinary export/reference consumers.
Differential tests cover suppression and restoration of unknown-echo diagnostics.

### Mixed workflow and limits

`*-region-before.json` and `*-incremental.json` use the original stdio probe,
three processes and seven samples per operation. They preserve the original
query/result checks and include the first edit restoring a missing stable ID
after the fix-all scenario. This semantic restoration makes the edit tail much
higher than steady prose typing; it has not been removed from the measurements.

| Operation | Large before | Large after | Medium before | Medium after |
| --- | ---: | ---: | ---: | ---: |
| Edit through diagnostics, median | 13.47 ms | 3.95 ms | 5.31 ms | 1.35 ms |
| Edit through diagnostics, p95 | 23.20 ms | 16.54 ms | 8.91 ms | 6.03 ms |
| Ordinary rename, median | 0.12 ms | 0.11 ms | 0.06 ms | 0.07 ms |
| Cancellation reply, median | 0.55 ms | 0.29 ms | 0.16 ms | 0.14 ms |
| Three-edit burst plus completion | 25.53 ms | 13.00 ms | 9.79 ms | 4.46 ms |
| Initial index readiness, median | 350.19 ms | 397.99 ms | 60.17 ms | 69.97 ms |
| Maximum observed RSS | 150.28 MiB | 143.25 MiB | 46.03 MiB | 42.97 MiB |

All successful query hashes match the original baseline, all cancellations return
`-32800` without edits, and only the final burst version is published. Cancellation
p95 was 1.72 ms and the maximum 2.20 ms on large, versus 0.66/0.68 ms before; the
median improvement is not a tail guarantee. Initial indexing regressed in these
small samples while edit latency and memory improved. Large before startup ranged
349–463 ms; after ranged 386–398 ms. No editor rendering latency is measured.

`region-edit-scaling.json` repeats the additional source/project layouts, one
fresh process and 21 samples per case. These are after-only scaling observations,
not paired comparisons with the older battery-powered scaling table.

| Project layout | Edited bytes | Prose | Newline | New block | Recovery |
| --- | ---: | ---: | ---: | ---: | ---: |
| 5,000 blocks / 5 files | 2,072,808 | 6.49 ms | 20.64 ms | 47.07 ms | 81.09 ms |
| 5,000 blocks / 80 files | 131,018 | 2.86 ms | 3.94 ms | 6.33 ms | 8.49 ms |
| 2,500 blocks / 10 files | 518,128 | 1.99 ms | 5.82 ms | 12.10 ms | 19.32 ms |
| 10,000 blocks / 40 files | 518,628 | 4.97 ms | 8.85 ms | 16.98 ms | 23.97 ms |

Very large individual files and structural/recovery edits remain more expensive.
The full-text transport, source scan, shifted output construction and project
membership updates still have costs proportional to their relevant input sizes.

### Final CPU evidence and verification

The separate profiling binary again uses release optimization, debug information,
unwind tables and frame pointers. Sampling attaches only after indexing and
opening the document, at 997 Hz with user-cycle events and DWARF call chains.
There were no lost samples; incomplete call chains still limit attribution.
`region-profiles.json` records binary/script hashes, sample timings and selected
inclusive shares. Raw local profiles are `/tmp/recite-regions-final-symbolized-*.data`.

For 500 comment edits, region analysis is 21.9% of sampled cycles and lowering
1.4%, versus changed-file analysis 63.9% and lowering 31.0% in the previous
profile. These shares describe different total runtimes and must not be treated
as wall-time savings. The instrumented median is 3.12 ms. Remaining work includes
workspace rebuilding, full-text handling and scanning the source for boundaries.

For 300 valid newline edits, analysis is 54.1%, with relocation at 19.0% and
summary composition at 13.2%; the instrumented median is 6.66 ms. These inclusive
shares overlap. Moving newly built region outputs into the final arrays, rather
than copying them again, is a candidate for reducing newline and cold-index
costs. It has not been implemented or claimed as a measured saving.

A separate 200-edit malformed-indentation workload measures diagnostic-heavy
publication explicitly. Its instrumented median is 7.14 ms; publication is 4.3%
of cycles, compared with 33.4% in the intermediate profile before source indexing
and cached sort keys. The 80-sample recovery profile has a 22.26 ms instrumented
median and still spends substantial time on local reanalysis and project checks.

The complete `mise exec -- just check` gate passed: 1,554 workspace tests with
three skips, editor/engine integrations, Clippy, rustdoc, documentation checks,
benchmark smoke, and both unchanged writer heap budgets. Two final recovery
regressions were additionally checked in the complete compiler unit suite
(21 tests) and incremental-validation integration suite (8 tests). Writer checks
and Clippy ran on the final implementation. Structural checks include all new
files and add no lint permissions. No commit, push or forge update was performed.
