# Authoring kernel comparison, 2026-09-16

Linux x86_64, AMD Ryzen AI 7 350 (8 cores / 16 threads), Rust 1.96.0, optimized
bench profile. Model-only measurements; no native compositor or GPU timing.

The baseline compiler is commit `df12b87dc8071a1eb825cfd18be0e3329b291973`.
A disposable checkout used that compiler with the **same corrected generator and
extended harness** as the after run. Runs were sequential, with 500 passages per
document and five per beat. Both versions assert zero initial diagnostics.

```sh
mise exec -- just bench-writer --passages 10000 --output /tmp/kernel-10k.json
mise exec -- just bench-writer --passages 100000 --output /tmp/kernel-100k.json
mise exec -- just bench-writer --passages 1000000 --output /tmp/kernel-1m.json
```

| Passages | Baseline | Optimised |
| --- | --- | --- |
| 10,000 | [before](before-10000.json) | [after](after-10000.json) |
| 100,000 | [before](before-100000.json) | [after](after-100000.json) |
| 1,000,000 | [before](before-1000000.json) | [after](after-1000000.json) |

Each report retains individual samples. Ten single-line wording edits and their
undo/redo operations exercise cached project validation. One multiline edit and
one stable-ID replacement, each followed by undo, exercise project invalidation.
History bytes are sampled after all these operations. Small sample counts and
single fallback timings are evidence, not stable percentile estimates or CI budgets.
See [the scalability report](../../scalability.md) for interpretation and remaining
work. The earlier repeated-name corpus is retained separately as historical
measurement evidence; it is not used as the baseline for the speedup here.
