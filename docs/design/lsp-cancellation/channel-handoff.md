# Crossbeam handoff experiment

## Question and acceptance

The native tracing and process-yield diagnostic identified macOS scheduling tails
at worker wakeup and writer handoff. The next experiment isolates those channel
operations before changing the language server. It compares Crossbeam 0.5.15's
blocking `Receiver::recv` with its public `Select::select` API, at the same queue
capacity and with the same work. The production coordinator already uses select.

The [blocking bounded receive source](https://docs.rs/crossbeam-channel/0.5.15/src/crossbeam_channel/flavors/array.rs.html)
uses `Backoff::snooze` before parking. The
[committed selection source](https://docs.rs/crossbeam-channel/0.5.15/src/crossbeam_channel/select.rs.html)
tries an operation, registers it, and parks. This is a source-derived hypothesis,
not evidence of a latency improvement. A one-arm `select!` macro is optimized
back to `recv`; the experiment deliberately uses the public `Select` object.
Rendezvous channels can still yield while completing their paired packet.

An existing [upstream proposal, Crossbeam #1251](https://github.com/crossbeam-rs/crossbeam/pull/1251),
checked on 6 October 2026, would add the same backoff to committed selection.
It remains open and unmerged. Its author's workload measures high-frequency
throughput and wakeup overhead, rather than this paced interactive latency. This
is a real tradeoff, and the current parking behaviour is an implementation
property of our pinned dependency, not a promise of the public API. Re-run these
measurements when updating Crossbeam; do not assume that selecting a receive
will always bypass yielding in future releases.

The probe is retained benchmark tooling, outside deterministic language code.
Its item-scoped clock exception follows the repository's benchmark suppression
policy. The proposed production slice changes only worker receives, preserves
capacity-one channels, and must pass cancellation, ordering, stale-result and
shutdown tests. It is retained only with consistent same-host recovery gains
and acceptable CPU cost in alternating real LSP session comparisons. An isolated
channel result is insufficient to retain a production change.

## Workload

`channel_handoff` exchanges numbered requests and replies between two native
threads. It checks every reply and records worker dispatch, reply receipt,
roundtrip and both send-call durations using one monotonic clock. There is a
requested 2 ms pause before each exchange, outside the measured latency. This
allows a worker to enter its idle receive path; actual pauses are recorded
separately because timer oversleep can let a yield depression expire before the
next request. Work is either zero or 200 microseconds of CPU spinning to
represent a short query. This bounded synthetic work is not a language benchmark.

The driver alternates blocking/selected ordering over three repetitions at
capacities zero and one, with 20 warmups and 150 measured exchanges per case.
On macOS it also runs both modes with the diagnostic `PTHREAD_YIELD_TO_ZERO=0`
setting. That setting stays confined to probe children. The expected diagnostic
signature is a disappearing default-versus-alternate dispatch gap for selected
capacity-one receives; capacity-zero packet waits may remain.

The child emits its report, then waits for acknowledgement. The driver samples
completed process CPU through psutil before acknowledging, including both native
threads and warmup. This avoids missing final CPU work by sampling after exit or
approximating it from periodic samples. Reports retain raw timings, exact sample
settings, binary hash, platform, CPU count and process CPU. Percentiles are
nearest-rank. Timeouts and incomplete reports fail the diagnostic job; no new
production timing budget is inferred from this synthetic probe.

## Reproduction

The optional channel step in the existing session workflow runs the exact same
benchmark on Ubuntu 24.04, Windows Server 2025 and macOS 15:

```sh
gh workflow run ci.yml --repo plethu/recite --ref feat/lsp-cancellation \
  -f lsp_sessions_only=true -f lsp_channel_probe=true
```

To run locally, build with `cargo bench --locked -p recite-benchmarks --bench
channel_handoff --no-run`, then supply the emitted executable path:

```sh
target/lsp-matrix-env/bin/python scripts/measure-lsp-channel-handoff.py \
  --binary target/release/deps/channel_handoff-HASH \
  --output target/lsp-channel/handoff.json
```

## Real-server candidates

The initial candidate registers one selected receive per analysis/query worker and reuses
it for each job. Channel capacity, send path, snapshots and coordinator selection
remain the same. `Select::select().recv()` is used explicitly; the single-arm
macro would undo the intended distinction. All 195 LSP tests passed locally,
including cancellation, shutdown, bounded queues and blocked-writer cases.

The optional server-control mode builds both revisions before any sampling and
drives both with the current harness. It alternates three fixed/churn repetitions
per binary, retains 210 post-warmup cycles each, and validates resource health,
fresh-server comparisons and portable completion/definition hashes. Historical
session hashes included temporary absolute URIs; the new fingerprint version
normalizes only the project URI prefix so target identity remains visible across
invocations. Comparison rejects older fingerprints and differing results,
drivers, fixtures or host settings. CPU measurements describe complete server
cycles, including analysis, rather than just channel overhead.

```sh
gh workflow run ci.yml --repo plethu/recite --ref feat/lsp-cancellation \
  -f lsp_sessions_only=true \
  -f lsp_server_control_ref=1f9c19a37e3088477809c6baa2c433cc3db5348f
```

The final decision and comparisons are recorded below.

## Initial isolated results

[Run 37508244379](https://github.com/plethu/recite/actions/runs/37508244379)
tested `1f9c19a3`. All three session jobs and `required-check` passed. The
[retained summary](channel-results.json) includes all cases, repetitions, CPU
measurements and binary identities; raw samples remain in Actions artifacts.

The table pools 450 measured exchanges per case, using default yield policy,
capacity one and 200 microseconds of synthetic query work:

| Host | Worker dispatch p95, blocking → selected | CPU per exchange, blocking → selected |
| --- | ---: | ---: |
| Linux | 0.041 → 0.039 ms | 0.196 → 0.176 ms |
| macOS | 5.434 → 0.065 ms | 0.306 → 0.288 ms |
| Windows | 0.018 → 0.018 ms | 0.276 → 0.123 ms |

Short Linux/Windows probes approach the OS CPU-accounting resolution; these CPU
numbers are not precise efficiency ratios. The real-server cycles supply the
more useful CPU comparison.

On macOS, the alternate-yield diagnostic also removed the capacity-one blocking
tail (pooled dispatch p95 0.054 ms with synthetic work). Selection kept all six
default-path capacity-one repetition p95s below 0.1 ms, but blocking's pooled
tail was dominated by the first repetition: individual p95s ranged from 0.04 to
10.02 ms without work and 0.10 to 9.41 ms with work. This supports the receive
backoff hypothesis while preserving its scheduling variability. The refined
probe records actual pauses to investigate that variability.

The bare rendezvous cases did not reproduce the earlier writer-handoff tail:
macOS default-path pooled dispatch p95s stayed below 0.06 ms in both receive
modes. They omit stdio flushing, message destruction and server contention. This
experiment therefore supports a bounded-worker change; it does not establish a
transport replacement or a universal Crossbeam defect.

Local `mise exec -- just check` passed at the worker prototype: 1,572 workspace
tests, three existing skips, writer/editor/engine checks, Clippy, dependency
policy, docs and benchmark smoke. The timer-field refinement additionally passed
targeted benchmark Clippy and workflow checks. The 43 CI unit tests passed,
including portable target identity and failed comparison evidence.

The existing `check-lsp-performance.py` gate passed locally against the exact
control/candidate release binaries in three alternating pairs, with 21 samples
per operation and matching output fingerprints. It covered all full/negotiated
edit and query/action cases, shared-file invalidation, startup/opening and RSS.
No workload crossed its regression policy, so no confirmation round was needed.
Binary identities, policy and per-pair workload medians are retained in the
summary. This is local Linux evidence; the hosted matrix supplies the platform
session comparison.

## Narrowing the worker change

All three jobs and `required-check` passed in
[paired run 37510297571](https://github.com/plethu/recite/actions/runs/37510297571).
The macOS job
tested `eb468a44` against `1f9c19a3`. Untraced recovery median/p95 fell from
12.17/52.35 ms to 10.40/33.52 ms, but median server CPU per cycle rose from
456.12 to 519.60 ms (14%). Five of six workload p95s improved; one low-tail
repetition rose from 13.38 to 13.95 ms. The native trace puts completion worker
wakeup p95 at 0.040 ms and completion response p95 at 0.504 ms. Cancelled rename
still has a 9.84 ms writer-handoff p95, confirming that this change does not
remove the whole transport tail. Tracing still perturbs scheduling and CPU.

That result motivates a narrower candidate: use selected receives only for the
short query worker. Leave analysis's receive path at the control implementation
and measure whether the request gain survives with less analysis CPU. All 195
LSP tests and targeted Clippy passed for this variant.

[Run 37513738763](https://github.com/plethu/recite/actions/runs/37513738763)
tested the query-only variant `341e1939`. All three hosts and `required-check`
passed, including the rendered editor and resource/fresh-server checks.
On macOS it removed the short completion tail (10.30 → 0.69 ms pooled p95), with
essentially unchanged median CPU (447.70 → 446.49 ms). Total recovery remained
variable: median rose from 14.61 to 19.44 ms, pooled p95 fell only from 50.59 to
48.19 ms, and individual workload medians/p95s changed in both directions.
Linux/Windows remained close to their controls. The preserved results support
the targeted request benefit but do not establish a consistent overall gain.
These two candidate experiments used different hosted runs; their absolute
values must not be treated as a direct comparison between variants.

## Decision, 6 October 2026

Retain selected receives for **both** workers. This restores the exact worker
implementation tested in `eb468a44`, whose paired recovery evidence was stronger.
The query-only variant is rejected as the production choice because its overall
recovery result was mixed. Its code and measurements remain in `341e1939` and
the retained summary.

| Host, both-worker comparison | Recovery median, control → candidate | Recovery p95, control → candidate | Median server CPU/cycle, control → candidate |
| --- | ---: | ---: | ---: |
| Linux | 6.61 → 6.72 ms | 8.08 → 8.10 ms | 490 → 500 ms |
| macOS | 12.17 → 10.40 ms | 52.35 → 33.52 ms | 456.12 → 519.60 ms |
| Windows | 16.06 → 16.05 ms | 21.51 → 19.76 ms | 890.63 → 890.63 ms |

The retained macOS result is a latency/CPU tradeoff: 36% lower pooled recovery
p95 and 14% more median and total measured server CPU during this burst workload.
The default yield policy is preserved. No busy wait, thread-priority override,
transport buffer change or new production dependency is introduced. Energy and
CPU under ordinary human-paced editing were not measured. The experiment does
not establish a complete macOS scheduling fix or a ranking against other LSPs.

The refined channel probe confirms substantial timer oversleep on macOS: a
requested 2 ms pause commonly became 13–18 ms. That can change whether a request
arrives during backoff or after the worker has parked: a plausible explanation
for the variability, rather than direct timing of individual yield calls.
Selected capacity-one receives kept repetition p95s below 0.1 ms, with occasional
larger isolated outliers. Real-server timing, rather than synthetic throughput or
requested pause duration, decides the production change.

CI now separately bounds the **post-cancellation completion stage** at 5 ms p95
on all three hosts. As with total recovery, an overrun must recur in two of three
repetitions for each fixed/churn workload. The retained candidate repetition p95s
were 0.37–1.10 ms; the unchanged macOS control exceeded 10 ms in two repetitions
of each workload. Replaying those actual reports through the new CLI passes all
candidate hosts and exits 1 for the control. Missing, negative or non-finite
measurements fail closed; unit tests also preserve tolerance of one isolated
overrun. The 20/40/75 ms overall budgets and 500 ms hard bound remain in force.
This prevents a future Crossbeam backoff change from hiding the request tail
inside the larger macOS total-recovery envelope.

The full local verification gate also passed for the narrower variant and new
guard: 1,572 tests and three existing skips, with all standard writer, editor,
engine, dependency, documentation and benchmark checks. After restoring the exact
retained worker code, all 195 LSP tests and targeted Clippy passed again. The
44 CI unit tests pass. `ci-scope.py` remains a cohesive 333-line lane-selection
module; its single added routing line passed the maintainability review trigger.

The remaining native hotspot is the cancelled-rename writer handoff and time
outside the instrumented server, each around 10 ms at p95. The bare rendezvous
probe did not reproduce that tail, so a realistic stdio/framing/destruction probe
would be the next justified investigation before changing transport or adopting
a different framework. No upstream report was published.

## Final integration

[Run 37518009003](https://github.com/plethu/recite/actions/runs/37518009003)
tested retained revision `bf003a2c`. All three platform jobs and `required-check`
passed with the completion budget wired into ordinary CI. Each host completed
six default-environment 40-cycle workloads, protocol and live-fault checks, fresh
server comparisons, resource health and the rendered editor lifecycle probe.
The retained summary includes the actual recovery/completion gate outputs and
untraced distributions from this final run. This unpaired validation establishes
integration health; its absolute timings are not a replacement for the earlier
same-job control/candidate comparison.
