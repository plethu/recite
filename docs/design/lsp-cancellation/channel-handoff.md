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

## Real-server candidate

The candidate registers one selected receive per analysis/query worker and reuses
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

Results and the retain/reject decision follow once measured.
