# Bounded LSP session and platform checks

The session test exercises accumulated work, rather than leaving an idle process
running. One server survives each fixed-working-set or document-churn workload.
The generated project has 40 documents, 800 blocks, 16,000 stable-ID lines and ten
shared destinations. No user project is edited.

Each cycle opens a document, introduces and repairs a broken reference, sends
5 ms edit bursts with competing completion and cancelled rename requests, saves,
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

`check-lsp-session-faults.py` drives a real child that retains touched memory,
threads and open files and progressively delays its replies. The same sampler and
decision function must detect all four faults on every OS. Unit tests cover warming caches, one-time
steps, isolated peaks, incomplete observations, and growing metrics.

## Running it

```sh
python3 -m venv target/lsp-matrix-env
target/lsp-matrix-env/bin/python -m pip install -r scripts/lsp-session-requirements.txt
mise exec -- cargo build --locked --release -p recite-lsp -p recite-fixturegen
target/lsp-matrix-env/bin/python scripts/measure-lsp-endurance.py \
  --binary target/release/recite-lsp --output target/lsp-sessions/fixed.json
target/lsp-matrix-env/bin/python scripts/measure-lsp-endurance.py \
  --binary target/release/recite-lsp --output target/lsp-sessions/churn.json --churn
```

On Windows, use the virtual environment's `Scripts/python.exe` and the server's
`.exe` suffix. Defaults are 40 cycles with 50 edits each. The initial extended
Linux check used 60 cycles with 100 edits each: 12,000 edits across both workloads
in about 100 seconds, stable resource counts, and matching fresh-server results.
The local VS Code probe also passed 20 document lifecycle cycles and 21 rendered
error/clear measurements. Hosted results are recorded separately after execution.

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
