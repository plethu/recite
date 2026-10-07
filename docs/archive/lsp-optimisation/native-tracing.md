# Native latency attribution and ecosystem choices

Historical evidence archived from `docs/design/lsp-cancellation/native-tracing.md` at `58b8f04965af`.
This records its named revision and execution profile; it is not current
workflow or implementation authority. Historical commands use their original
revision and paths. See the [archive index](../README.md).

The subsequent [resource and ownership study](resource-tradeoff.md) tests the
actual pinned stdio topology, standard worker inputs and JSON ownership. It
retains the existing protocol owner and transport; no framework migration or
thread-priority override is earned by the measured results.

## Decision, 5 October 2026

The job is responsive, correct editing of Recite projects on Linux, macOS and
Windows. Incremental text transactions, cancellation, fresh diagnostics, bounded
queues and deterministic results remain required. This investigation addresses
repeated recovery tails, not a new language-server framework.

The ecosystem already supplies the commodity pieces. Recite uses rust-analyzer's
`lsp-server`, `lsp-types`, Crossbeam and Serde. For native observability this change
adopts `tracing` 0.1.44 and `tracing-subscriber` 0.3.23, with standard JSON output
and monotonic uptime timestamps. It does not introduce a trace format writer,
async runtime or replacement stdio transport.

| Candidate / precedent | What it supplies | Decision and reason |
| --- | --- | --- |
| [rust-analyzer's lsp-server](https://rust-lang.github.io/rust-analyzer/lsp_server/index.html) | Synchronous protocol framing, initialization and channel transport | Keep the existing dependency. Current upstream still uses zero-capacity reader/writer/dropper channels; a newer release is not an evidenced scheduling fix. |
| [tracing-subscriber](https://docs.rs/crate/tracing-subscriber/0.3.23) | Structured events, filtering, JSON formatting and monotonic timestamps | Adopt. MIT licensed; documented Rust 1.65 minimum is below this workspace's toolchain. No Tokio runtime is needed. |
| [async-lsp](https://docs.rs/async-lsp/0.2.4/async_lsp/) | Tower middleware for lifecycle, routing, concurrency limits, cancellation and tracing | Deferred after released-version saturation probes; see [dependency decisions](dependency-decisions.md). Recite's snapshot and publication rules still need an owner. |
| [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server) | Typed asynchronous handlers, transport and request cancellation | Deferred after ordering/cancellation probes and the deletion inventory; see [dependency decisions](dependency-decisions.md). No framework migration is retained. |
| [lsp-textdocument](https://docs.rs/lsp-textdocument/0.5.0/lsp_textdocument/) | Document storage and position conversion using our current lsp-types generation | Not a direct replacement for our transaction boundary: source inspection shows surrogate-interior positions rounding down and updates applied sequentially, with an assertion for reversed ranges. Recite rejects malformed batches atomically. Adopting it would still require that validation layer. |
| [Salsa](https://salsa-rs.github.io/salsa/overview.html) | Dependency tracking and incremental query recomputation | Relevant if manual dependency invalidation becomes the measured bottleneck or maintenance burden. It does not replace protocol transport, and these recovery tails do not establish that need. |
| [rust-analyzer thread intent](https://rust-lang.github.io/rust-analyzer/src/stdx/thread/intent.rs.html) | Platform scheduling policies for worker and latency-sensitive threads | Useful prior art, not a justification for raising priorities without attribution. No QoS override is retained. |

The document-manager assessment used its [update and position-conversion source](https://docs.rs/lsp-textdocument/0.5.0/src/lsp_textdocument/text_document.rs.html), checked on 6 October 2026.

These are source/documentation assessments, not comparative benchmark claims
about alternative frameworks. The stop condition for custom infrastructure is a
candidate that preserves the existing protocol tests while removing material
local machinery or improving paired measurements. If tracing identifies a
transport defect, reproduce it against the existing upstream component before
considering a fork. An async framework should earn its migration cost through
that same test contract; popularity alone does not measure the remaining gap.

Sources checked include the architecture and source of rust-analyzer, current
Tower LSP and async-lsp APIs, Salsa documentation and tracing package metadata.
The inspected [upstream stdio implementation](https://github.com/rust-lang/rust-analyzer/blob/master/lib/lsp-server/src/stdio.rs)
retains the same channel arrangement as our pinned 0.7.9. rust-analyzer also has
[Windows main-loop priority handling](https://github.com/rust-lang/rust-analyzer/blob/master/crates/rust-analyzer/src/main_loop.rs),
which is evidence that platform scheduling deserves measurement even in mature
servers; it is not evidence for a particular Recite fix.

## Reproduction and interpretation

Set `RECITE_LSP_TRACE_DIR` to an existing directory to opt in. The binary creates
one new `<pid>.jsonl` file there. Existing files are not overwritten. Without the
variable it installs no subscriber. Embedding users can install their own tracing
subscriber; the library does not change global logging configuration. Events
contain method names, request IDs, job serials and revisions, not source text,
paths, diagnostics or result payloads. Protocol stdout remains unchanged.

```sh
mkdir -p target/native-trace
RECITE_LSP_TRACE_DIR="$PWD/target/native-trace" target/release/recite-lsp

gh workflow run ci.yml --repo plethu/recite --ref BRANCH \
  -f lsp_sessions_only=true -f lsp_native_trace=true
```

The matrix alternates traced/untraced ordering over three repetitions of both
fixed and churn workloads on the same release binary and fixture. Normal CI
keeps tracing off. The ordinary recovery gate applies to the untraced workloads;
traced workloads retain semantic, resource and hard recovery assertions. All
reports, traces and the native summary are retained in Actions artifacts.

The summarizer joins client and server by process/request identity and subtracts
durations, never timestamps from unrelated clock epochs. It focuses on the
cancelled rename and completion at the end of each burst, excluding five warmup
cycles per workload. Worker serials distinguish dispatch, wakeup, execution and
result receipt. Native ingress is after the dependency's reader channel; handoff
is observed after the writer channel send. The residual includes uninstrumented
transport and Python scheduling. It can be negative if the sending thread resumes
after the driver receives the reply; it is preserved rather than clamped away.
The trace does not claim to time the dependency's actual stdout flush.

Trace output uses synchronous file writes. Enabled/disabled comparisons quantify
its disturbance; native boundaries locate waits, but traced timings are not
substitutes for ordinary user-facing timings. In particular, logging can alter
thread scheduling. No scheduler or channel-buffering change is bundled into the
attribution experiment.

## macOS yield-path diagnostic

The first trace locates roughly 10 ms tails at worker wakeup and writer handoff,
while completion execution p95 remains below 0.2 ms. This motivates a process-only
causal probe, not a change to Recite's scheduling policy.

[Crossbeam's backoff](https://docs.rs/crossbeam-utils/0.8.21/src/crossbeam_utils/backoff.rs.html#218)
calls `thread::yield_now`, which
[Rust 1.96 implements through sched_yield](https://github.com/rust-lang/rust/blob/1.96.0/library/std/src/sys/thread/unix.rs).
[Apple's libpthread source](https://github.com/apple-oss-distributions/libpthread/blob/main/src/pthread.c)
selects a priority-depressing yield by default, and reads `PTHREAD_YIELD_TO_ZERO`
at process startup. Setting it to `0` selects the alternate non-depressing yield
path. That implementation switch is not treated as a supported product API.

The session driver accepts `--server-yield-to-zero 0` solely for diagnosis. It
passes the setting to server children, including fresh oracles; it does not
change Python's environment or the shipped editor launcher. Reports record the
child override. To compare against the unchanged driver on the same binary:

```sh
gh workflow run ci.yml --repo plethu/recite --ref BRANCH \
  -f lsp_sessions_only=true -f lsp_native_trace=true \
  -f lsp_macos_yield_probe=true \
  -f lsp_driver_control_ref=c7fba9a6372c937fac6bf5e4452d654e7643660c
```

The override applies only to the macOS candidate workloads. All ordinary CI and
product launches retain their existing environment. The baseline remains the
historical driver with default server settings; tracing-enabled candidates
separately show whether the implicated boundaries change.

## Native attribution results

[Run 37386141235](https://github.com/plethu/recite/actions/runs/37386141235)
tested `c7fba9a6`. All three platform jobs and `required-check` passed, including
native trace protocol tests, six untraced and six traced workloads per OS,
resource/fresh-server comparisons, the untraced recovery gate and rendered editor
checks. Each tracing mode contributed 210 post-warmup recovery samples and 210
samples of each recovery request. The [retained summary](native-results.json)
contains binary/fixture identity, full phase distributions and CPU measurements.

| Host | Recovery median, untraced → traced | Completion execution p95 | Completion worker wake p95 | Cancelled rename writer handoff p95 |
| --- | ---: | ---: | ---: | ---: |
| Linux | 4.01 → 4.00 ms | 0.138 ms | 0.020 ms | 0.010 ms |
| macOS | 20.37 → 24.58 ms | 0.163 ms | 9.989 ms | 9.882 ms |
| Windows | 16.09 → 15.05 ms | 0.101 ms | 0.037 ms | 0.030 ms |

MacOS's traced completion response p95 was 10.33 ms, despite sub-millisecond
execution at p95. Cancelled rename response p95 was 20.02 ms; both native writer
handoff and the residual outside the instrumented server path had roughly 10 ms
tails. These phase percentiles are not additive. A few execution outliers also
remain, including a 31.5 ms completion; elapsed execution spans include any
preemption during execution.

Tracing perturbed macOS recovery: median rose around 21%, while pooled p95 moved
from 53.38 to 54.51 ms. Therefore the trace locates plausible scheduling boundaries
but does not replace the untraced comparison. No production performance gain is
claimed from adding observability.

`mise exec -- just check` passed with a task-owned temporary directory on disk:
1,572 workspace tests, three existing skips, writer checks, editor/engine checks,
Clippy, dependency policy and benchmark smoke. The initial environment attempts
failed because `/tmp` filled and then because target-isolation fixtures reject a
temporary target inside the repository. Neither required a source workaround.

## Yield diagnostic result and next decision

[Run 37388009511](https://github.com/plethu/recite/actions/runs/37388009511)
tested `df886b73` on macOS 15. The job and required check passed. Three alternating
rounds compared the default server environment with `PTHREAD_YIELD_TO_ZERO=0`,
using the same release binary, fixture hashes, seed and Python switch interval.
Each untraced mode supplied 210 measured recovery samples after warmup; the
traced candidate supplied another 210. All 18 workloads completed their 40 cycles
and passed diagnostics, stable-result, fresh-server and resource assertions.
The rendered editor check also passed with the default environment.

| Untraced measurement | Default yield | Alternate yield |
| --- | ---: | ---: |
| Recovery median | 32.94 ms | 7.81 ms |
| Recovery pooled p95 | 55.48 ms | 15.27 ms |
| Recovery maximum | 63.65 ms | 19.86 ms |
| Six workload p95 range | 52.68–56.18 ms | 13.12–18.76 ms |
| Server CPU median per cycle | 356.65 ms | 504.21 ms |

Recovery median fell 76% and p95 fell 72%, consistently across fixed and churn
repetitions. CPU per cycle rose 41%; the test does not establish whether that is
extra scheduling work or more analysis completing before coalescing/cancellation.
It does not establish an energy improvement. Workload wall times remained
similar because edit pacing and the rest of the document lifecycle dominate.

The alternate-path trace puts completion worker-wakeup p95 at 0.032 ms,
cancelled-rename writer-handoff p95 at 0.033 ms, and their response p95 values at
0.379 ms and 0.290 ms respectively. The earlier default-path trace had roughly
10 ms tails at those boundaries. That phase comparison comes from separate
hosted runs; the untraced table above is the same-job comparison. Tracing still
perturbs scheduling: candidate recovery median/p95 were 7.01/11.02 ms with tracing
versus 7.81/15.27 ms without it.

This is strong causal evidence for the server's macOS yield policy contributing
to the tails. It does not isolate one Crossbeam call site: the environment setting
affects every yield in the server process. No production default or CI budget
was changed. Ordinary CI still uses default scheduling and enforces the existing
20/40/75 ms repeated-p95 budgets on Linux/Windows/macOS, plus the 500 ms hard
recovery assertion and resource-growth gates.

The [follow-up channel experiment](channel-handoff.md), completed on 6 October,
reproduced the bounded-worker delay and retained Crossbeam's public committed
selection API for both workers. Its same-job macOS comparison reduced recovery
p95 from 52.35 to 33.52 ms and the post-cancellation completion stage from
10.27 to 0.51 ms, at 14% more median server CPU per burst cycle. A query-only
variant was measured and rejected for mixed overall recovery results. CI now
also enforces a separate 5 ms repeated-p95 completion budget.

The cancelled-rename writer handoff remains a measured hotspot. A realistic
stdio/framing/destruction reproduction would be the next useful experiment;
the bare rendezvous probe did not reproduce its tail. Later released-version
[dependency probes](dependency-decisions.md) defer `async-lsp` after a saturation
stall and defer Tower after the contract and deletion assessment. Neither
framework migration nor Salsa is justified as an immediate performance fix.
The undocumented environment switch remains diagnostic only.
