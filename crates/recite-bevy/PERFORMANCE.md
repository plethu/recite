# Headless performance probe

Run `CARGO_TARGET_DIR=<disk-backed target> scripts/probe-bevy-performance.sh`
from the repository root. The script builds the CPU-only Bevy example once,
compiles the published runtime-surface fixture outside the measured commands,
then times separate child processes with Python's external wall timer and
`wait4` resource usage. There are no CI thresholds. The report is retained at
`$CARGO_TARGET_DIR/recite-bevy-probe/performance.txt`.

Observed on 2026-09-28, Rust/Cargo 1.96.0, Bevy 0.19.1, Linux x86_64,
AMD Ryzen AI 7 350, debug profile:

| Probe | Work | Wall time | Peak RSS |
| --- | ---: | ---: | ---: |
| Decode and validate | 10,000 compiled asset conversions, 3,266-byte input | 1.861 s | 13,720 KiB |
| Idle | 10,000 `App::update` calls, no session | 0.504 s | 13,848 KiB |
| Active | 1,000 start/choice/blocking ack/choice/end cycles; 1,000 condition dispatches | 0.426 s | 15,204 KiB |
| Retained revision | One active refresh and next-session start | 0.009 s | 15,092 KiB |

The retained-revision probe observes two distinct compiled revision identities
while the old session and new asset cache coexist. Public APIs do not expose
the underlying allocation or `Arc` counts, so the probe does not claim a
byte-accurate retained-memory figure. Wall/RSS figures include process startup
and vary by host, profile, and fixture.
