# LSP experience and regression-gate follow-up

Historical evidence archived from `docs/design/lsp-cancellation/follow-up.md` at `58b8f04965af`.
This records its named revision and execution profile; it is not current
workflow or implementation authority. Historical commands use their original
revision and paths. See the [archive index](../README.md).

Local experiments on 2026-10-05, using the release server from `d7c4306c`.
This report separates server throughput, installed-client latency and automated
regression protection. None establishes a best-in-class or cross-platform claim.
Raw observations are in [follow-up](follow-up/).

## Sustained sessions

`scripts/measure-lsp-session.py` sends edits independently of response arrival,
with completion, definition, cancelled rename and fix-all requests competing for
service. It records successful and stale/cancelled replies separately, diagnostic
versions, send lateness, outstanding client requests and Linux process RSS. The
last version must settle and subsequent completion, definition and rename must
succeed. These are full-text comment edits, not all possible writing actions.

| Session | Completion outcomes | Cancellation | Diagnostics |
| --- | --- | --- | --- |
| 518 KB source, 6,000 edits, 20 ms interval | 1,200/1,200 succeeded; all-response p95 20.46 ms | 546/546 cancelled; p95 1.38 ms | 5,997 versions; final version delivered |
| 2 MB source, 1,000 edits, 20 ms interval | 200/200 succeeded; all-response p95 23.23 ms | 91/91 cancelled | 1,000 versions delivered |
| 2 MB source, 1,000 edits, 5 ms interval | 0/200 succeeded; all became stale | 91/91 cancelled | 516 versions; final version delivered |

All fix-all requests became stale while continuous typing proceeded. These
errors are safe freshness outcomes, but they are not successful low-latency
actions. The aggressive run's maximum send lateness was 41.91 ms, so its offered
load was not a perfectly sustained 200 Hz. At most four client requests were
outstanding in these sessions; this is not an internal server queue measurement.
The two-minute run's RSS plateaued at 139,524 KiB for its final ten observations.
This is evidence against growth in this workload, not a general leak proof.

## Ranged-sync prototype and production integration

The disposable [patch](ranged-sync-prototype.patch), against `d7c4306c`, advertises
incremental sync and normalizes UTF-16 ranges into full text on the coordinator
before existing coalescing. It retains an additional text copy per open document.
Normalizing before coalescing preserves the dependency chain between edits.
The prototype measurements below precede the production integration described at
the end of this report. The patch remains as historical experiment evidence.

Three alternating process pairs per layout, 21 recorded samples per workload,
two warmups, identical diagnostic hashes and unchanged power metadata:

| Workload | 518 KB full / ranged | 2 MB full / ranged |
| --- | ---: | ---: |
| Prose | 3.46 / 3.11 ms | 9.50 / 6.36 ms |
| Newline insertion | 6.66 / 5.82 ms | 19.15 / 16.13 ms |
| Stable-ID label | 5.88 / 5.24 ms | 16.63 / 13.86 ms |
| New block | 16.72 / 19.02 ms | 57.85 / 61.84 ms |
| Recovery | 27.05 / 28.16 ms | 102.23 / 104.06 ms |

Range construction happens before the edit timer. These results measure sending
an already-built edit through diagnostics; they exclude editor diff generation.
The experiment's whole-string materialization still has linear costs. Unicode,
CRLF, rejection of mid-surrogate positions, stale versions, 50 dependent edits
without diagnostic waits and close/reopen were checked against full-sync results.
This does not establish production readiness for every malformed range or client.
The server-only comparison does not justify promotion: it adds coordinator
ownership and regresses structural edits. The installed-host experiment below
changes the priority, because the client's full-sync batching dominates latency.
That finding prompted a production implementation with atomic range validation
and explicit protocol-text ownership before the coalescing queue.

## Large-file profiles

Separate `perf record -e cycles:u -F 997 --call-graph dwarf,16384` samples attached
to the owned server after opening/indexing. Each profile drove 100 recorded
edits plus two warmups on the 2 MB source. Both reported zero lost samples.
The retained text reports use self-cycle percentages, not additive wall-time
savings. Release symbols are available, but libc frames and call chains are
incompletely resolved.

Persistent-map lookup accounted for 18.90% of self cycles on new-block edits
and 11.13% on recovery. Localisable-ID validation accounted for 5.45% and 2.19%,
respectively. Recovery also includes header scanning and syntax-node construction.
These support investigating repeated validation/index lookups next; they do not
justify changing the persistent representation without another bounded comparison.
The measured roughly 58 ms structural and 102 ms recovery medians remain limits.

## Installed Neovim client: retained debounce improvement

`scripts/measure-lsp-neovim.lua` drives keyboard edits through the installed
Neovim 0.12.5 adapter and measures through application to its diagnostic store.
It uses a headless host with Tree-sitter disabled and then requests completion.
It measures neither screen painting nor a full syntax-rendering workload.

The host defaults to a 150 ms change debounce when none is configured. Three
alternating pairs on the 518 KB source gave these median diagnostic-store times:

| Pair | 150 ms setting | 50 ms setting |
| --- | ---: | ---: |
| 1 | 131.90 ms | 34.31 ms |
| 2 | 131.94 ms | 33.45 ms |
| 3 | 131.55 ms | 30.20 ms |

The cadence alternates an edit, diagnostics and completion: some of the debounce
interval elapses during completion. These are not universal per-keystroke delays.
Recite now defaults to 50 ms and passes through `lsp.flags`; flags participate
in client reuse and survive crash recovery. Users can explicitly restore 150 ms.

## Installed VS Code/VSCodium clients

Both pinned official Linux hosts passed the existing installed-VSIX contract,
keyboard workflow, clean-shutdown and process-leak checks. A separate headless
Wayland probe alternates small end-of-file changes between a valid target and an
unknown target, and waits for the corresponding diagnostic-store event. It then
requests completion through the real host provider. It records 21 observations
after two warmups, with binary/source hashes and host identity.

An initial whole-buffer replacement probe was discarded as a model of small
editor edits; those observations remain separately labelled in the evidence.
Three alternating pairs of fresh VS Code profiles measured:

| Pair | Full-sync diagnostic median | Ranged-sync diagnostic median |
| --- | ---: | ---: |
| 1 | 235.17 ms | 71.34 ms |
| 2 | 238.14 ms | 72.71 ms |
| 3 | 238.15 ms | 73.27 ms |

Completion-provider medians ranged from 36.47–43.82 ms with full sync and
35.28–37.30 ms with ranged sync. These are small local host samples rather than
a release tail guarantee or screen-paint measurement.
VSCodium reproduced the direction: 235.47 versus 72.81 ms for diagnostics,
and 42.20 versus 38.20 ms for completion. The same production server and the
same prototype binary were used in both hosts.

The installed `vscode-languageclient` implements a 250 ms pending-change delayer
for full-text synchronization (`lib/common/client.js`,
`triggerPendingChangeDelivery`); incremental synchronization sends ranged events
directly (`lib/common/textSynchronization.js`). That is a concrete reason to
prioritize production incremental sync despite the server-only tradeoff above.
The prototype changes both transport and coordinator work, so the host timing
does not isolate the delayer as the sole source of the observed difference.
Production hardening should cover invalid multi-change transactions, position
boundaries, schema overlays and bounded work during update/cancellation bursts.
These measurements used the isolated prototype; production verification follows below.

## Regression protection

The tiny Criterion smoke now includes LSP. Its change-refresh workload now makes
an actual comment edit instead of resending identical bytes. The deeper stdio
comparison covers six edit kinds in both full and negotiated sync modes, plus
eight query/action kinds on the large
seed-7203 fixture. Both release binaries are built before measurement.

`scripts/check-lsp-performance.sh BASE_COMMIT` selects the baseline explicitly.
CI uses the merge base with the PR base or pre-push commit; scheduled/manual
runs use the preceding commit. This catches change-local regressions, not the
accumulation of many individually small regressions against a fixed release.
Policy lives in `scripts/lsp-performance-policy.json`:

- Three alternating pairs, two warmups and 21 recorded samples per workload.
- Both a 20% relative increase and a 2 ms absolute increase are required.
- At least two pairs and the median of the three per-process medians must fail.
- A suspected regression triggers another three pairs; recurrence confirms it.
- Different failing workloads between rounds are inconclusive and fail closed.
- Missing observations, changed fixtures, differing output fingerprints or changed
  execution-profile metadata fail rather than silently accepting the comparison.

The same-binary A/A run and the complete build/compare/session shell command
against `HEAD` passed. A test-only stdio proxy adding 10 ms to each outbound
message failed all 14 original workloads in both rounds and exited with status 1.
Those observations cover the original 14-workload policy; negotiated-sync
coverage was added during production integration. Unit tests also cover incomplete evidence, omitted workloads,
different output/fixture hashes, isolated noise and invalid timings.
Raw data includes all observations rather than only accepted medians.
The absolute tolerance intentionally leaves submillisecond
operations without a tight relative timing gate. Startup, RSS, cancellation and
burst measurements are retained, but are not additional comparative budgets.
The sustained-session CI probe checks behaviour and settlement under load.

The workflow retains JSON artifacts even on failure. Hosted Ubuntu variability
still needs confirmation in actual CI; local calibration does not establish a
hosted false-positive rate. Release/cross-platform budgets remain with #109.

## Reproduction

```sh
mise exec -- cargo build --locked --release -p recite-lsp -p recite-fixturegen
target/release/recite-fixturegen --profile large \
  --output target/lsp-performance/large --summaries target/lsp-performance/fixture.json
python3 scripts/measure-lsp-session.py target/lsp-performance/large \
  --edits 6000 --output target/lsp-performance/session.json
python3 scripts/measure-lsp-edit-workloads.py target/lsp-performance/large \
  --output target/lsp-performance/edits.json
mise exec -- bash scripts/check-lsp-performance.sh BASE_COMMIT
```

The historical ranged prototype can be applied to a disposable checkout at
`d7c4306c` with
`git apply docs/design/lsp-cancellation/ranged-sync-prototype.patch`; build its
server and pass `--ranged --binary PATH` to the edit probe. The current production
server also accepts `--ranged` probes directly. Keep production and
prototype processes sequential. For the host probe, set absolute
`RECITE_PERF_ROOT`, `RECITE_PERF_BINARY` and `RECITE_PERF_OUTPUT`, use isolated
XDG state/cache/config directories, then run
`nvim --headless --clean -l scripts/measure-lsp-neovim.lua`.
`RECITE_PERF_DEBOUNCE` is an optional experimental override.

For a verified official VS Code/VSCodium extraction and an already built VSIX,
`scripts/measure-lsp-vscode.sh HOST_BINARY PROJECT RELEASE_LSP OUTPUT_JSON`
uses a temporary profile and headless Wayland compositor. Its alternating valid
and unknown-target edits force observable diagnostic-store changes; its completion
measurement includes the real host provider. These differ from the Neovim
comment-edit workload and must not be used to rank the editors against one another.

## Verification and delivery

The complete `mise exec -- just check` gate passed: 1,563 workspace tests with
three existing skips, writer tests and unchanged heap budgets, Clippy, editor and
engine integration, documentation and the expanded benchmark smoke. The final
29 CI-policy tests, workflow lint, shell/JavaScript syntax checks and diff checks
also passed. The new host probe ran successfully in both official hosts; the
VS Code full/ranged comparison repeated across three pairs.

Working-tree review included untracked source files. The three production/tooling
files above the 250-line scrutiny threshold remain cohesive: `recite.lua` is the
plugin setup facade; `ci-scope.py` owns lane selection; `measure-lsp-latency.py`
owns the existing stdio probe, now with explicit failure cleanup. No changed
production/tooling file exceeds 400 lines and no lint permissions were added.

Delivery is local changes and recorded evidence. No commit, push, hosted CI run
or publication was performed. Windows/macOS host performance and perceptual
screen-rendering acceptance remain unmeasured. The production server now advertises incremental sync; the original prototype
patch remains historical evidence, not the production implementation.

## Production incremental synchronization

The working tree now advertises incremental UTF-16 synchronization. Protocol
text and versions have a dedicated owner in `server/text_sync.rs`, independent
of cancellable analysis. Sequential changes are applied to a candidate string;
only a fully valid transaction advances the accepted text/version or invalidates
queries. The queue receives full snapshots, preserving its existing coalescing
and lifecycle barriers. Full replacements remain supported. This retains one
additional text copy per open document and linear range/materialization work;
it is not a rope or an incremental parser redesign.

Boundary tests cover non-BMP text, LF/CRLF/CR, overlong character clamping,
invalid surrogate boundaries, reversed ranges, invalid lengths, sequential
batches and rejection without consuming a version. The clamping rule follows
[LSP 3.17 Position](https://github.com/microsoft/language-server-protocol/blob/gh-pages/_specifications/lsp/3.17/types/position.md).
Protocol tests exercise ranged schema overlays and 100 dependent edits mixed
with cancellation, followed by a semantic edit and close/reopen.

The initial production full-text comparison passed all 14 original workloads.
Production ranged medians on the 518 KB / 2 MB sources were: prose 3.26 / 6.17 ms,
newline 6.07 / 15.95 ms, block topology 20.03 / 61.63 ms and recovery 28.54 /
103.78 ms. These are one process per layout, not another paired estimate.
A 1,000-edit ranged session on the 2 MB source at 20 ms intervals delivered all
1,000 diagnostic versions. Completion succeeded 197/200 times; three replies
were stale. All 91 cancelled rename requests settled as cancelled. Completion
all-response p95 was 22.33 ms, cancellation p95 4.32 ms, maximum send lateness
5.69 ms, and final RSS 164,836 KiB. Timed ranged events replace only the known
EOF suffix, so the driver does not scan the whole source to construct each edit.

The regression policy now has 20 workloads: six full-text edit workloads, six
negotiated-sync edit workloads and eight query/action workloads. The negotiated
mode follows each binary's advertised capability, permitting comparisons with
older full-sync releases. Full and ranged sustained sessions are both required.


The first expanded gate failed negotiated block-topology edits in both rounds:
16.05 → 19.37 ms and 15.56 → 19.54 ms. The evidence is retained in
`production-before-tuning.json`. Resolving each end position relative to the
already resolved start removes a redundant scan of the unchanged prefix. The
20-workload direct comparison then passed, followed by a passing complete
build/compare/session shell run. In the final three pairs, median-of-medians
for negotiated block-topology edits was 16.99 → 19.57 ms; recovery was
28.07 → 29.36 ms. Structural cost remains, below this gate's threshold in the
final run; the optimization does not establish a universal speedup. Full and
ranged 300-edit CI sessions each delivered all 300 diagnostic versions and
settled successfully.

Verification ran the complete repository gate through 1,567 passing workspace
tests (three existing skips). Clippy identified an unused schema-test import;
after removing it, Clippy and all remaining gate components passed, including
Rustdoc, writer tests/heap budgets, documentation and benchmark smoke. Writer
verification required unsandboxed local socket/subprocess access. After range
scan tuning, the complete LSP test suite and Clippy passed again, followed by
four focused text-sync boundary tests including mixed full/ranged batches and
clamped ranges. No new lint permissions were introduced.


Fresh installed-host pairs with the production binary reproduced the prototype's
client improvement: VS Code diagnostic-store median 239.38 → 72.88 ms and
VSCodium 237.17 → 74.77 ms. Completion medians were 34.79 → 35.55 ms and
37.04 → 40.97 ms respectively. Neovim with the new 50 ms debounce on both sides
remained similar (34.15 → 35.80 ms diagnostic-store median); the earlier
150 → 50 ms debounce experiment remains the evidence for its principal gain.
These final checks used 21 recorded samples after two warmups per host/binary,
fresh isolated profiles, and the production release binary. They do not measure
screen rendering or establish parity on untested platforms. One background
VSCodium request logged the expected stale-snapshot error during editing; the
probe's diagnostic and completion assertions and clean host exits all passed.

The final 20-workload gate also rejected the test-only 10 ms outbound-message
slowdown with exit status 1: 19 workloads exceeded the policy in the first round
and 20 in the second. No thresholds were relaxed. The retained JSON contains
both failing rounds and all raw samples. Hosted CI execution and its runner
variance remain unverified; the workflow wiring and failure behavior were
verified locally. Delivery remains local edits only, with no commit or push.

The [validation and rendered-editor continuation](continuation.md) records the
subsequent production changes, expanded 24-measurement gate, and final evidence.
