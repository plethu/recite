# LSP validation, overload recovery, and rendered-editor follow-through

This continues the [initial follow-up](follow-up.md). The changes and measurements
are local to `feat/lsp-cancellation`; no commit, push, hosted CI result, or
cross-platform acceptance is implied.

## Production changes

Project validation shares overlapping contexts across affected documents. Batches
are bounded at 3,072 passages to avoid expanding a whole large project's compact
facts into temporary AST nodes at once. A single target still receives its complete
required context even if that context exceeds the batch budget. Membership indices
use persistent hash tries; published diagnostics and other observable ordering
remain explicitly sorted.

The first unbounded batching experiment raised startup peak RSS from roughly
153 MiB to 193 MiB on the large-file fixture. That experiment was rejected in
favour of bounded batches. The first 16,384-passage cap still exceeded the
writer's fixed 20 MB heap budget (28.1 MB observed); lowering the cap to 3,072
passages preserves room below that existing budget. A 4,096-passage experiment
passed at 19.95 MB but was rejected for its narrow margin. A regression test exercises 17 independent thousand-line
files and checks the largest materialized batch. The shared-destination test also
checks that a hundred affected documents are validated once each.

Fix-all requests that have no missing stable IDs now return before constructing
the project-wide occupied-anchor set. Completeness and malformed/ambiguous input
checks still run first, including errors outside the requested file. Completion
formats the common localized block-kind label once per response.

## Regression policy

The paired gate now checks 24 measurements: six edit types in both full and
negotiated sync modes, eight queries, a shared-reference fanout fixture, process
startup/index readiness, opening a document, and process peak RSS. Fanout uses
100 documents, 2,000 blocks, 20,000 lines, and ten shared destination files.

Each side has 21 observations after two warmups in each of three alternating
pairs. Suspected regressions trigger another three pairs. Response fingerprints
and fixture identities must agree. Warm latency requires both a 20% and 2 ms
increase; readiness uses 30% and 50 ms, opening uses 30% and 10 ms, and peak RSS
uses 20% and 16 MiB. A repeat-confirmed regression or inconsistent failing rounds
fails the gate. Missing or mismatched evidence also fails closed.

Startup samples use fresh processes with a warm filesystem. Peak RSS is Linux
`VmHWM` after opening a document; it is not a long-lived heap-leak budget. The CI
wrapper also runs steady sessions and a burst session with 25 edits at 5 ms
spacing, pauses of at least 150 ms, and a 500 ms recovery limit. Recovery requires
fresh diagnostics, completion, definition, rename, and fix-all results.

## Rendered-editor method

The optional installed-host probe connects only to the isolated host's temporary
DevTools endpoint. It waits for the expected visible source and error decoration
state, then two animation frames, before capturing the compositor surface. It
records decoration-frame latency separately from screenshot completion. The latter
includes capture overhead. Neither measures physical display scanout.

The first error and clear frames are retained locally for inspection. Screenshots
of installed proprietary hosts are not added to Recite's source distribution.

## Final branch-baseline comparison

The complete shell gate passed against `d7c4306c` with no suspected regressions.
Values below are medians of three process medians, with 21 observations per
process. The unchanged full-sync measurements separate compiler work from the
negotiated-sync editor path.

| Measurement | Baseline | Candidate |
| --- | ---: | ---: |
| Shared-reference structural edit | 102.81 ms | 30.40 ms |
| No-op fix-all | 12.36 ms | 0.51 ms |
| Completion | 7.37 ms | 5.93 ms |
| Actual missing-ID fix-all | 12.59 ms | 12.90 ms |
| Full-sync new block | 12.48 ms | 11.17 ms |
| Full-sync recovery | 19.87 ms | 18.65 ms |
| Fresh-process index readiness | 411.00 ms | 386.12 ms |
| Opening a document | 2.47 ms | 2.29 ms |
| Peak RSS after opening | 121,416 KiB | 123,808 KiB |

Small negotiated edits incur roughly 0.5–1 ms extra server-side range-processing
cost on this fixture; the installed-client improvement comes from avoiding full
buffer serialization and transfer. Actual missing-ID fix-all is essentially
unchanged. These are measured tradeoffs, not universal speedups.

[Raw paired comparison](continuation/comparison.json) and the
[full-sync](continuation/session.json), [ranged](continuation/session-ranged.json),
and [burst](continuation/session-bursts.json) sessions are retained. All 11 burst
recovery checks passed; the maximum was 17.61 ms. Ranged steady-session RSS was
129,636 KiB after 50 edits and 130,140 KiB after 300 edits. This bounded observation
does not establish a long-lived leak guarantee.

## Compiler-only and large-file experiments

A separate three-pair comparison uses the retained pre-validation incremental-sync
binary as control, with ranged sync on both sides. Medians of process medians:

| Workload | Previous incremental implementation | Final implementation |
| --- | ---: | ---: |
| Standard fixture new block | 13.23 ms | 11.68 ms |
| Standard fixture recovery | 21.03 ms | 19.13 ms |
| 2 MB-file new block | 47.29 ms | 43.52 ms |
| 2 MB-file recovery | 82.36 ms | 78.14 ms |
| Shared-reference new block | 103.07 ms | 32.01 ms |

[Raw isolated pairs](continuation/final-validation-pairs.json) retain binary hashes,
fixture identities, and all samples. The control is an intermediate local build,
not a separately committed revision. The committed branch baseline comparison
above remains the reproducible CI comparison.

The [2 MB-file steady session](continuation/final-narrow-steady.json) delivered all
1,000 diagnostic versions and all 200 completion results at 20 ms edit spacing.
Six definition requests and one fix-all request were correctly rejected as stale;
91 explicitly cancelled renames returned cancellation errors. Peak RSS was 155,652 KiB after
startup and 159,488 KiB after the final edit. The
[500-edit burst session](continuation/final-narrow-bursts.json) passed all 19
recovery checks, with a maximum of 19.18 ms. Cancellation and stale-result
rejection remain enabled; uninterrupted 5 ms editing can still invalidate every
request before completion, as the earlier overload experiment showed.

## Fault-injection result

The normal comparator rejected the same binary with 10 ms stdout delays and
64 MiB retained memory: exit status **1**, `status: regression`, with the same
23 failing measurements in both rounds. Peak RSS failed in both rounds. Startup
readiness stayed within its distinct 50 ms/30% tolerance. Protocol fingerprints
still matched. No thresholds were changed for this test.

[Both failing rounds](continuation/fault-comparison.json) and
[fault provenance](continuation/fault-provenance.json) are retained. This validates
local failure behavior; it is not a claim that the modified workflow has already
run on a hosted CI runner.

## Final installed-host rendering results

VS Code uses three alternating pairs; VSCodium uses one corroborating pair.
Each process records 21 edits after two warmups. VS Code values are medians of
three process medians. Both use official installed Linux hosts, the current
packaged Recite extension, isolated profiles, and the same generated source.

| Host and measurement | Baseline | Final |
| --- | ---: | ---: |
| VS Code diagnostic store | 317.75 ms | 64.58 ms |
| VS Code visible decoration + two frames | 344.47 ms | 91.79 ms |
| VS Code screenshot completed | 435.18 ms | 182.89 ms |
| VS Code completion provider | 27.05 ms | 23.46 ms |
| VSCodium diagnostic store | 317.38 ms | 64.87 ms |
| VSCodium visible decoration + two frames | 338.96 ms | 89.57 ms |
| VSCodium screenshot completed | 434.24 ms | 185.97 ms |
| VSCodium completion provider | 25.04 ms | 24.14 ms |

[Rendered summary](continuation/rendered-summary.json) and the eight adjacent
`render-vscode-*.json` / `render-vscodium-*.json` files retain all observations.
Error/clear screenshots were inspected locally. Screenshot completion adds roughly
90 ms of measurement overhead and must not be reported as decoration latency.
These rendered runs have a different cadence from the earlier non-rendered probes;
compare control and candidate within the same experiment.

## Verification and scope

The repository verification stages passed across the original run and targeted
resumption: 1,570 workspace tests (three existing skips), doctests, Clippy,
editor/adapter suites, writer tests, docs, dependency/policy checks, and benchmark
smoke. After tightening the batch cap, all 461 compiler/LSP tests and their Clippy
checks passed again; both writer heap gates, docs, and benchmark smoke passed.
The linked writer heap workload peaks at 18,592,584 bytes under its unchanged
20,000,000-byte budget. All 31 CI contract tests pass.

The initial full invocation needed disk-backed temporary storage; the resumed
project checks also needed the scoped maintainability tool environment. The writer
heap failure was a real implementation regression and was fixed before the final
performance runs. It was not treated as environmental noise.

This closes the planned local validation, typing/recovery, memory, and rendered-host
experiments. The workflow is wired to fail required CI on confirmed large
regressions, and that failure path was exercised locally. Hosted-runner calibration,
macOS/Windows rendering, physical-display latency, and comparisons with other
language servers are not established by this evidence. No universal best-in-class
claim is made. Delivery remains local edits only.

## Reproduction

```sh
mise exec -- bash scripts/check-lsp-performance.sh HEAD

RECITE_PERF_RENDER=1 mise exec -- bash scripts/measure-lsp-vscode.sh \
  /path/to/official/host /path/to/generated/project \
  target/release/recite-lsp /path/to/output.json

cc -shared -fPIC -O2 -Wall -Wextra -Werror \
  scripts/lsp_fault_injection.c -o /path/to/lsp-fault.so
```

For the negative control, use an executable wrapper that sets `LD_PRELOAD` to the
fault library and `exec`s the same release binary used as control. The library
retains 64 MiB and delays stdout writes by 10 ms. Run the normal comparator with
that wrapper as candidate. The injected process preserves protocol output so
both the latency and RSS decisions can be tested without weakening fingerprints.
