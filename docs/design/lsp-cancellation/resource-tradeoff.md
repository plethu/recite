# Worker scheduling resource tradeoff

The initial selected-worker change exchanged lower macOS recovery tails for
approximately 14% more process CPU in one 200 Hz burst session. Later paced
comparisons show mixed CPU costs. The current refinement uses standard bounded
worker inputs and moves ready JSON once into the response and writer handoff.
No everyday energy claim follows from these workload measurements.

## Bounded experiments

The optional `lsp_resource_probe` input on the existing CI workflow runs:

- Three alternating control/candidate repetitions of fixed/churn sessions at
  100 and 250 ms edit intervals. Each uses 20 lifecycle cycles and ten edits per
  cycle, preserving result fingerprints, fresh-server comparisons and resource
  health checks. These are chosen pacing profiles, not a claim about all human
  editing. The default CI contract remains 40 cycles and 50 edits at 5 ms.
- A three-second settled idle interval in every session, sampled with process
  CPU accounting after the timed cycles. CPU during the editing/drain/repair
  stage is opt-in for resource experiments, separately from CPU during the full
  lifecycle cycle. Whole-process CPU also includes initialization and checkpoint
  queries. Accounting resolution and host scheduling limit small differences;
  use totals/means as well as medians, especially on Linux and Windows.
- A native benchmark using the pinned `lsp-server` stdio transport unchanged,
  with real Content-Length framing, diagnostics immediately before a small
  response, and zero, 128 or 4096 diagnostic entries. A direct single-threaded
  framing baseline helps isolate transport scheduling; it is not a compatible
  production replacement. Both variants use the same consuming Python client.

The stdio benchmark records coordinator send-return intervals on one Rust
clock and client receipt on the driver clock. Send return is not writer receive
or flush: the sender can remain descheduled after the writer accepts a message.
Internal spans alone cannot justify a production change. Client JSON parsing
and reader scheduling also contribute to wire timing, especially for the large
payload. Preserve individual samples rather than adding unrelated percentiles.

## Architecture decision boundaries

Retain the existing protocol/freshness owner, immutable snapshots and two
bounded workers. A framework replacement must first demonstrate ordered
notifications, final cancellation/freshness checks, bounded output under a
blocked consumer and shutdown on all supported hosts. A migration is not earned
by the current scheduling evidence.

If ordinary pacing shows a material CPU increase, attribute completed and
cancelled analysis work before introducing an edit-coalescing timer. Faster
dispatch can change how much work starts, so whole-cycle CPU cannot identify
channel overhead by itself. Optional counters require an enabled/disabled
measurement to establish that instrumentation does not change the result.

If the complete stdio probe reproduces the real tail, isolate writer message
destruction before changing transport ingress. The upstream writer writes and
flushes a message, then synchronously hands it to a dropper before receiving
the next message. Inline destruction and a capacity-one disposal queue are
possible diagnostic ablations; writer ingress must keep the current commitment
boundary. If the probe does not reproduce the tail, instrument the actual
dependency in a temporary diagnostic build before proposing a custom transport.

Stop scheduling experiments once the chosen paced profiles have acceptable CPU,
idle CPU is negligible, and client-visible tails meet the existing budgets
without an unexplained regression. No energy claim follows from process CPU.

## Output ownership prototype

The coordinator previously cloned ready response JSON twice and diagnostic JSON
once before each selection, including iterations that handled ingress or worker
completion instead of sending. The Crossbeam macro evaluates its send payload
expression only after the send operation wins. Choose a request ID or queued
publication first, then move its JSON inside that expression. This retains the
same operation priorities and requires no new selection loop or transport.

No input is accepted between selection and packet completion. Pending state
therefore remains the authority for cancellation and freshness until the
selected send consumes it. Keep that interval small: a rendezvous writer may
already be waiting for the packet. Native tracing retains its metadata-only
`output_ready` and send-return `handoff` boundaries; it does not claim to observe
writer receive. A blocked-writer fixture now includes a large queued response.

Measure this independently against the parked-worker checkpoint before
combining it with any worker-channel experiment. Functional correctness and
removing allocations do not by themselves establish a latency improvement.

The local standard gate passed three alternating pairs with matching result
hashes and no suspected regression. Large-fixture completion medians improved
in all pairs: 8.650 to 6.715 ms, 9.024 to 7.253 ms and 8.776 to 6.813 ms
(19.6–22.4%). Other workloads remained within the established regression policy.
The compact per-pair evidence is in `resource-results.json`; this result alone
does not establish cross-platform recovery or CPU behaviour.

Completed queries already contain owned `serde_json::Value`. Pinned
`lsp-server` 0.7.9's generic `Response::new_ok` converts its argument to JSON,
traversing and reallocating an existing value. An independent refinement can
put that value directly in `Response.result`. Keep `Some(Value::Null)` distinct
from an absent result, preserve the ID and error fields, and retain the same
Running/Ready acceptance and cancellation boundaries. Measure it separately
from worker inputs and stopped-error preparation before retaining it.

That isolated local comparison passed all standard workloads with matching
result hashes and no suspected regression. Large completion medians changed
from 6.576 to 3.712 ms, 7.940 to 3.631 ms and 6.477 to 3.331 ms (43.6–54.3%
lower). Formatting, all LSP tests and targeted Clippy passed. The complete local
gate passed for the preceding stopped-response checkpoint: 1,572 workspace
tests, three existing skips, and all writer/editor/dependency/documentation and
benchmark checks. Final integrated validation also passed below; these are local
paired gains, not an OS ranking.

The final retained combination (`3c4dae15`, without stopped-error prebuilding)
also passed the standard three-pair gate against the parked-worker checkpoint
`47a20c79`, with matching results and no suspected regression across its 24
workloads. Large completion medians changed from 8.995 to 3.733 ms, 9.127 to
3.480 ms and 8.586 to 3.423 ms (58.5–61.9% lower). Hover, fix-all, lifecycle and
memory workloads remained within the established regression policy.

## Standard worker input prototype

Each worker input has exactly one receiver and is never part of a coordinator
selection. A capacity-one `std::sync::mpsc::sync_channel` can therefore replace
these two inputs while leaving selectable Crossbeam results and stdio channels
intact. Disconnect and join ownership stay the same. This removes the special
single-operation `Select` receive without adding a dependency.

[Rust 1.96's bounded receive](https://github.com/rust-lang/rust/blob/1.96.0/library/std/src/sync/mpmc/array.rs)
registers and parks after an empty attempt, whereas pinned Crossbeam bounded
receive snoozes first. This is an implementation distinction, not a guarantee
that every standard-channel operation is yield-free. Partial slot publication
and rendezvous completion can still back off. The open
[Crossbeam selection backoff proposal](https://github.com/crossbeam-rs/crossbeam/pull/1251)
also illustrates why the existing selection workaround's scheduling behaviour
must remain protected by measured completion budgets.

Compare standard inputs against the deferred-payload checkpoint so the two
effects remain separable. Retain them only with semantic/hash parity, equivalent
resource behaviour and passing completion/recovery tails on all three hosts.
Neither this experiment nor the ownership change changes protocol queue bounds.

Retain standard inputs as a single-consumer API simplification with mixed
performance. The [same-job comparison](https://github.com/plethu/recite/actions/runs/37529023882)
against deferred payloads passed all three platform jobs and the required
rollup. It included protocol/editor checks, resource health, completion/idle
budgets, paced profiles and enabled/disabled native tracing.

| Host | Cycle CPU total at 5 ms | At 100 ms | At 250 ms | Stress recovery p95, control → standard inputs |
| --- | ---: | ---: | ---: | ---: |
| Linux | +1.49% | -1.59% | +3.47% | 8.14 → 8.07 ms |
| macOS | -5.58% | +15.83% | +4.43% | 9.72 → 7.97 ms |
| Windows | +0.87% | +0.25% | +3.17% | 18.19 → 18.34 ms |

The macOS 100 ms increase remains material: whole-session CPU increased 16.68%,
corroborating cycle accounting. Pair changes were +10.7/+73.5/-10.1% fixed and
+5.9/+26.2/-6.1% churn; most excess was concentrated in two repetitions. This
is neither a universal CPU penalty nor evidence of no regression. Rename wire
outliers over 10 ms fell from 13 to five of 90 samples, while pooled p95 stayed
near 20 ms. At 250 ms they fell from four to zero; stress fell from two to zero
of 210. All six ordinary macOS completion p95s were 0.23–0.42 ms and idle CPU
was zero. These are whole-workload costs, not channel overhead or energy use.
The final integration matrix cannot erase the unresolved paced CPU tradeoff.

## Measurement audit and idle regression gate

The first resource study, [run 37522227256](https://github.com/plethu/recite/actions/runs/37522227256),
passed all three hosts. On macOS, cycle CPU totals changed by -4.45% for stress,
-10.87% at 100 ms and +6.07% at 250 ms. Individual workload pairs were mixed;
this is not evidence of a universal CPU penalty or its disappearance. The
blocking control's completion p95 was also fast (0.417 ms), despite matching
the exact binary hash from the earlier slow run. OS/Python identities match,
but physical hosts and scheduling conditions are not proven identical.

That study inserted two extra CPU reads around editing and a startup idle
interval. The second read sat between repair receipt and cancelled rename send,
so it could change scheduling at the measured transition. Default latency CI
now disables those reads and measures idle after the timed cycles. Resource
profiles explicitly opt into editing CPU accounting; driver identity records
sampling, idle duration and idle location. Missing stage CPU remains absent.

The optional `lsp_driver_accounting_probe` runs only the exact supplied control
binary on macOS, with three alternating enabled/disabled fixed-workload pairs.
It holds startup idle at three seconds in both arms, disables native tracing,
and requires a clean committed driver, equal results, fresh-server checks and
one fixture/environment/binary identity. Wire timings come from joined client
send/receive timestamps; report stage timings and repair-to-rename gaps
separately. Do not attribute earlier differences to accounting unless the
within-job audit reproduces that effect.

The completed [accounting audit](https://github.com/plethu/recite/actions/runs/37529046162)
reproduced the slow control's completion wire p95 at 10.24–10.42 ms in all
three disabled/enabled pairs. Median recovery differences were mixed; both
arms retained the tail. These CPU reads do not consistently remove the stall,
and cannot explain the earlier fast run. Applying the actual 5 ms completion
budget to the three disabled repetitions rejected all three (report-stage p95
10.36–10.61 ms). Keep the default latency driver free of the optional reads.

Every ordinary cross-platform session gate additionally rejects more than
100 ms of process CPU during a three-second settled interval in two of three
repetitions of either working set. This conservative bound catches sustained
idle work above about 3.3% of one core. All 18 candidate intervals in the first
normal matrix reported zero CPU; the largest control value across its paced
and stress profiles was below 0.04 ms on macOS, with coarse zero readings on
Linux/Windows. Missing/invalid accounting and intervals shorter than 2.5 seconds
fail closed. The gate protects idle CPU, not active-work CPU or energy use.

## Stopped-response placement experiment

The deferred-payload [comparison](https://github.com/plethu/recite/actions/runs/37524857541)
completed measurements, protocol/editor checks and latency gates on all three
hosts. The Windows job subsequently exceeded its 45-minute timeout while
saving the Rust cache, after artifact upload; the overall run is cancelled.
The measurements remain available, but this is not a green workflow result.

At 100 ms pacing on macOS, deferred payloads exposed cancelled-rename wire
tails in five of six workloads, versus one of six for the control: 11 versus
four of 90 post-warmup samples exceeded 10 ms. Completion wire tails stayed
below 2.3 ms. These samples justify an isolated placement test, not a claim
that the rendezvous backoff caused every delay.

The optional `lsp_stopped_response_probe` builds only small stopped/error
responses before selection; large ready JSON still moves in the chosen arm.
If another arm wins, the error is discarded and recomputed from authoritative
pending state. It compares three alternating fixed/churn pairs at 100 ms,
40 cycles and ten edits, holding editing CPU accounting and startup idle at
the settings that exposed the concern. Both binaries use standard worker
inputs, so placement is the only server difference. Report each repetition's
actual wire tails and count over 10 ms, separately for cancelled and successful
rename outcomes, alongside CPU and recovery. A fast control makes the original
selected-worker concern inconclusive. Retain this placement
only if the repeated cluster improves without undoing large-response gains.
If it does not, stop packet-placement tweaks and require writer-side evidence.

The [completed placement study](https://github.com/plethu/recite/actions/runs/37532570495)
exercised cancelled responses in all 210 measured requests per binary, with no
successful rename races. Counts above 10 ms changed from 7/0/7 fixed and 7/0/0
churn to 1/6/0 and 3/7/0 respectively: 21 to 17 in total, but tails remained in
three of six workloads for each variant. Two pairs became substantially worse.
Cycle CPU increased 0.66% and whole-session CPU 1.93%; recovery median increased
8.27% while pooled p95 decreased 12.90%. Reject the prebuild as a production
optimization: the repetition evidence is mixed and the cluster did not disappear.
The frozen checkpoint and outcome-aware probe remain reproducible. Stop further
packet-placement experiments; no custom transport is justified by this result.

The actual upstream stdio probe did not reproduce the small-message 10 ms stall:
macOS native response handoff p95 stayed below 0.131 ms for empty diagnostics and
below 0.289 ms for 128 entries. At 4096 entries, native handoff reached 3.25 ms,
but native/direct wire tails were similar (about 26.5–29.3 ms), including Python
JSON consumption. These results do not justify a custom production transport.

## Final integration and stopping decision

[Run 37534833894](https://github.com/plethu/recite/actions/runs/37534833894)
tested the final code at `3c4dae15`. All Linux, macOS and Windows jobs and the
required rollup passed. Each host completed six ordinary 40-cycle/50-edit
workloads, fresh-server comparisons, protocol and live-fault checks, resource
health and the rendered VS Code lifecycle probe. Native tracing and editing CPU
reads were disabled; idle was measured after the timed cycles.

| Host | Largest recovery repetition p95 | Largest completion repetition p95 | Settled idle CPU across six repetitions |
| --- | ---: | ---: | ---: |
| Linux | 7.67 ms | 0.23 ms | All reported 0 ms |
| macOS | 45.02 ms | 0.64 ms | All reported 0 ms |
| Windows | 23.85 ms | 0.47 ms | All reported 0 ms |

The complete local gate passed for this final code: 1,572 workspace tests, three
existing skips, all 48 CI contract tests, and standard writer, editor, engine,
dependency, documentation and benchmark checks. Timing above establishes
unpaired integration health; it does not rank hosts or erase the paced CPU cost.
The durable summary retains source/binary/driver identities and actual gate,
health, fault and editor results. Full traces and rendered frames remain in
Actions artifacts for 30 days.

Retain standard bounded worker inputs and both JSON ownership improvements.
Reject stopped-error prebuilding and further speculative packet-placement work.
Keep the existing transport, coordinator, queue bounds and freshness ownership;
no new production dependency, QoS override or framework migration is earned.
The remaining macOS scheduling variability and paced CPU tradeoff are explicit
limits of this evidence. Continue from release feedback or a concrete reproduced
regression, rather than an open-ended pursuit of a best-in-class claim.

The subsequent [final resource investigation](final-resource-profiling.md) adds
direct allocation attribution and retains exact-sized region composition and
completion reservation. It preserves these scheduling and ownership decisions.
