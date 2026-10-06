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
  CPU accounting. CPU during the editing/drain/repair stage is reported
  separately from CPU during the full lifecycle cycle. Accounting resolution
  and host scheduling still limit interpretation of small differences.
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
