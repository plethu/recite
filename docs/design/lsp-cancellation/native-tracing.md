# Native latency attribution and ecosystem choices

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
| [async-lsp](https://docs.rs/async-lsp/0.2.4/async_lsp/) | Tower middleware for lifecycle, routing, concurrency limits, cancellation and tracing | Strongest alternative if we replace more protocol orchestration. Keep as a migration candidate, not a demonstrated latency improvement. Recite's snapshot and publication rules still need an owner. |
| [tower-lsp-server](https://github.com/tower-lsp-community/tower-lsp-server) | Typed asynchronous handlers, transport and request cancellation | Viable framework alternative. Its transport documents that concurrency one disables cancellation; adopting it still requires deliberate ordering and CPU-work offloading. No migration in this experiment. |
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

[Crossbeam's backoff](https://github.com/crossbeam-rs/crossbeam/blob/crossbeam-utils-0.8.21/crossbeam-utils/src/backoff.rs)
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
  -f lsp_macos_yield_probe=true -f lsp_driver_control_ref=c7fba9a6
```

The override applies only to the macOS candidate workloads. All ordinary CI and
product launches retain their existing environment. The baseline remains the
historical driver with default server settings; tracing-enabled candidates
separately show whether the implicated boundaries change.
