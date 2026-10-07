# Headless performance probe

Run `CARGO_TARGET_DIR=<disk-backed target> scripts/probe-bevy-performance.sh` from the repository
root. The script builds the CPU-only Bevy example once, compiles the published runtime-surface
fixture outside the measured commands, then times separate child processes with Python's external
wall timer and `wait4` resource usage. There are no CI thresholds. The report is retained at
`$CARGO_TARGET_DIR/recite-bevy-probe/performance.txt`.

The [September 2026 observations](../../docs/archive/delivery-evidence.md#bevy-performance) record
one host/profile. Rerun the probe for the candidate under investigation; those values are not a
current baseline or regression budget.
