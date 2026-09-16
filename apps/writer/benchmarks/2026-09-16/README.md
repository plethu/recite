# Writer workload measurements, 2026-09-16

Local working-tree results on Linux x86_64, AMD Ryzen AI 7 350 (8 cores / 16 threads),
Rust 1.96.0. See [the assessment](../../scalability.md) for interpretation and limits.

- [10,000 passages](project-10000.json), [100,000 passages](project-100000.json),
  [1,000,000 passages](project-1000000.json):
  `mise exec -- just bench-writer --passages N --output PATH`.
- [1,000 beats](scene-1000.json), [10,000 beats](scene-10000.json):
  `mise exec -- just bench-writer-ui N`.
- [Recovery](recovery.json): `mise exec -- just bench-writer-recovery`.

The JSON stores individual samples rather than a pass/fail timing threshold.
Project workloads use the optimized bench profile. GUI/recovery use the test
profile. The GUI samples were rerun sequentially after compilation and ordinary
component tests finished. No native compositor or physical input was measured.
Files are retained evidence, not golden test snapshots or CI performance budgets.
