# Worker scheduling resource tradeoff

The both-worker receive change currently exchanges lower macOS recovery tails
for approximately 14% more process CPU in the 200 Hz burst session. That result
does not establish an everyday editing or energy cost. Keep the production
choice provisional while measuring the narrower resource question.

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

Every ordinary cross-platform session gate additionally rejects more than
100 ms of process CPU during a three-second settled interval in two of three
repetitions of either working set. This conservative bound catches sustained
idle work above about 3.3% of one core. All 18 candidate intervals in the first
normal matrix reported zero CPU; the largest control value across its paced
and stress profiles was below 0.04 ms on macOS, with coarse zero readings on
Linux/Windows. Missing/invalid accounting and intervals shorter than 2.5 seconds
fail closed. The gate protects idle CPU, not active-work CPU or energy use.

The actual upstream stdio probe did not reproduce the small-message 10 ms stall:
macOS native response handoff p95 stayed below 0.131 ms for empty diagnostics and
below 0.289 ms for 128 entries. At 4096 entries, native handoff reached 3.25 ms,
but native/direct wire tails were similar (about 26.5–29.3 ms), including Python
JSON consumption. These results do not justify a custom production transport.
