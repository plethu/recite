# Bounded LSP session and platform checks

The session test exercises accumulated work, rather than leaving an idle process
running. One server survives each fixed-working-set or document-churn workload.
The generated project has 40 documents, 800 blocks, 16,000 stable-ID lines and ten
shared destinations. No user project is edited.

Each cycle opens a document, introduces and repairs a broken reference, sends
edits scheduled at 5 ms intervals with competing completion and cancelled rename requests, saves,
closes/reopens, restores the saved source, creates and deletes a temporary saved
document, and changes/restores the manifest content-set configuration. The fixed
workload reuses the temporary document identity; churn uses a new identity each
cycle. Both finish each cycle with identical project contents and one open file.

A versioned diagnostic is the settling barrier. Checkpoints compare project-wide
completion and definition fingerprints with the initial result. Every tenth cycle
and the final cycle additionally compare with a fresh server. Response timings
include seven samples for each checkpoint query. Reports and JSONL operation
traces include the seed and binary/fixture identity. Generated paths can differ
between runs; the seed and workload recreate the sequence.

## Health decisions

After five warmup cycles, split the remaining checkpoints into three windows.
Compare the window medians and the median of all pairwise slopes. Fail only when
both successive windows grow, the overall increase exceeds the absolute and
relative tolerance, and the slope is positive. A single cache step or isolated
peak is not classified as sustained growth.

| Metric | Absolute tolerance | Relative tolerance |
| --- | ---: | ---: |
| Current resident memory | 32 MiB | 20% |
| Thread count | 4 | Positive growth |
| File descriptors / Windows handles | 8 | Positive growth |
| Completion or definition median | 10 ms | 100% |

These are major-degradation tripwires, not proof against arbitrarily small leaks.
Resident memory is measured by pinned `psutil` on each platform, not compared
between operating systems. Recovery from the final burst edit, including draining
outstanding requests, is bounded at 500 ms; request/diagnostic waits
have deadlines, and each workload has a ten-minute cycle budget. The CI job also
has a 30-minute outer timeout including builds and the editor host.

CI repeats each workload three times. After five warmup cycles, the nearest-rank
p95 recovery must stay within 20 ms on Ubuntu 24.04, 40 ms on Windows Server 2025,
and 75 ms on macOS 15. Exceeding the platform limit in at least two repetitions
fails the job. A single repetition above this tail limit is recorded; the 500 ms
per-cycle limit remains unconditional. Missing, truncated or incomparable reports
fail closed. See [recovery calibration](recovery-calibration.md) for the measured
driver correction and the rationale for these hosted-runner budgets.

The post-cancellation completion stage has a separate 5 ms repeated-p95 budget
on every platform, with the same two-of-three rule. It measures the short
completion request after the burst repair and cancelled rename, including driver
send/receive/resumption. This protects the query-worker scheduling improvement
from being hidden inside macOS's larger total-recovery budget. The calibrated
candidate repetition p95s were below 1.1 ms; the unchanged macOS control exceeded
10 ms in two repetitions of each workload and is rejected by this gate.
See the [channel experiment](channel-handoff.md) for the comparison and CPU
tradeoff. Missing or invalid completion-stage measurements fail closed.

`check-lsp-session-faults.py` drives a real child that retains touched memory,
threads and open files and progressively delays its replies. The same sampler and
decision function must detect all four faults on every OS (latency exercises both
query metrics). Unit tests cover warming caches, one-time steps, isolated peaks,
incomplete observations, and growing metrics.

This complements the paired base-versus-candidate performance gate described in
[the optimisation report](continuation.md). Session checks detect degradation as
work accumulates; the paired gate detects a candidate that starts out slower.
Neither depends on a person keeping an editor open for hours.

## Running it

```sh
python3 -m venv target/lsp-matrix-env
target/lsp-matrix-env/bin/python -m pip install -r scripts/lsp-session-requirements.txt
mise exec -- cargo build --locked --release -p recite-lsp -p recite-fixturegen
for round in 1 2 3; do
  target/lsp-matrix-env/bin/python scripts/measure-lsp-endurance.py \
    --binary target/release/recite-lsp --output "target/lsp-sessions/fixed-$round.json"
  target/lsp-matrix-env/bin/python scripts/measure-lsp-endurance.py \
    --binary target/release/recite-lsp --output "target/lsp-sessions/churn-$round.json" --churn
done
target/lsp-matrix-env/bin/python scripts/check-lsp-session-recovery.py \
  --reports target/lsp-sessions --budget-ms 20 --completion-budget-ms 5 \
  --output target/lsp-sessions/recovery.json
```

On Windows, use the virtual environment's `Scripts/python.exe` and the server's
`.exe` suffix; choose the corresponding platform budget above. Defaults are 40
cycles with 50 edits each per invocation. The initial extended
Linux check used 60 cycles with 100 edits each: 12,000 edits across both workloads
in about 100 seconds, stable resource counts, and matching fresh-server results.
The local VS Code probe also passed 20 document lifecycle cycles and 21 rendered
error/clear measurements. Hosted results follow below.

## Hosted matrix

`.github/workflows/lsp-sessions.yml` runs on Ubuntu 24.04, Windows Server 2025 and
macOS 15. It builds the release server, runs the incremental stdio contract test,
proves live fault detection, and runs both session workloads. The official pinned
`@vscode/test-electron` runner launches VS Code 1.136.1 with the compiled Recite
extension and an isolated profile. It exercises rendered diagnostics followed by
20 document lifecycle cycles. The extension runs from the development directory;
this matrix is not a VSIX installation test. Existing package checks retain that
separate responsibility.

The matrix is selected for relevant compiler/LSP/editor and harness changes and
is included in `required-check`. Scheduled CI runs include it. To run only the
matrix and CI policy checks on a branch:

```sh
gh workflow run ci.yml --repo plethu/recite --ref BRANCH -f lsp_sessions_only=true
```

Each OS retains checkpoint reports, operation traces, and rendered frames as
30-day Actions artifacts, including on failure. Timing comparisons are within
one host/run; physical display timing and human usability are left to real-world
feedback after release.

## Initial results, 5 October 2026

These measurements predate the [driver pacing correction](recovery-calibration.md).
They remain historical evidence of the original session gate.

[Actions run 37351523403](https://github.com/plethu/recite/actions/runs/37351523403)
tested revision `4034c3f2`. All three platforms passed both 40-cycle workloads
(4,000 burst edits per OS), five fresh-server comparisons per workload, the live
fault control, and the incremental stdio contract test. Each VS Code host passed
20 document lifecycle cycles and 21 rendered diagnostic observations.

| Host | Both LSP workloads | Worst burst recovery | Median visible diagnostic frame |
| --- | ---: | ---: | ---: |
| Ubuntu 24.04, x64 | 55.7 s | 12.6 ms | 95.8 ms |
| macOS 15, arm64 | 154.2 s | 77.8 ms | 111.2 ms |
| Windows Server 2025, x64 | 87.8 s | 27.3 ms | 116.9 ms |

All workloads stayed within the resource-growth budgets and returned the same
completion/definition results as fresh servers. Timings describe these hosted
runs; different runner hardware prevents treating this table as an OS ranking.
[The retained summary](session-results.json) includes resource windows, binary
hashes, workload settings and fault outcomes. Raw checkpoints, traces and frames
are available in the linked run's artifacts for 30 days.

The matrix exposed two Windows tooling defects that were fixed before this run:
Mise implicitly installed unrelated repository tools when Cargo was invoked, and
Git's CRLF conversion made generated editor projections fail their byte-for-byte
freshness checks. The lane now disables implicit tool installation; Git attributes
preserve LF in those generated files. A simulated CRLF checkout also passed the
extension build locally. The macOS fault control needed a stronger injected
delay to cross the existing relative threshold; acceptance limits were unchanged.
