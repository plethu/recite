---
title: Benchmarks
description: Commands and policy for Recite benchmark smoke checks and regression review.
---

Recite benchmarks live in `crates/recite-benchmarks` and use Criterion against the shared synthetic
fixture profiles and checked-in realistic fixture packs. The suite is split into explicit compiler,
runtime, preview, and LSP bench targets.

## Fast smoke

Use the smoke script for pull-request and CI checks that need to prove the benchmark targets still
build and execute quickly:

```bash
just perf smoke
```

The script uses the checked-in tiny fixture. Criterion `--test` executes cases without saving
baselines or comparing timings; it establishes buildability and execution only.

The CI benchmark lane also runs a separate paired LSP regression check:

```bash
just perf setup
just perf compare BASE_COMMIT
```

This builds both revisions and samples them on the same runner. The
[checked-in policy](https://github.com/plethu/recite/blob/main/scripts/lsp-performance-policy.json)
owns exact workloads, sample counts, thresholds and confirmation rounds. It requires matching output
fingerprints and complete evidence; raw samples and binary identities remain artifacts under
`target/lsp-performance/`. These Linux process checks do not establish cross-platform rendering,
cold-storage startup or arbitrary-session memory budgets.

## Full suites

Run all bench targets with the local default scale set:

```bash
cargo bench -p recite-benchmarks
```

Without `RECITE_BENCH_SCALES`, the benchmark crate runs `tiny,small`. Select heavier generated
profiles explicitly when release review or profiling needs them:

```bash
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks
RECITE_BENCH_SCALES=large,epic cargo bench -p recite-benchmarks -- --sample-size 10
```

The preview target is the exception: its no-environment default is the checked-in `tiny` fixture
plus `realistic:v1-pack`, so traversal evidence covers both a generated shape and an authored
project.

The same selector also accepts checked-in realistic packs:

```bash
RECITE_BENCH_SCALES=realistic:v1-pack cargo bench -p recite-benchmarks
RECITE_BENCH_SCALES=tiny,realistic:v1-pack cargo bench -p recite-benchmarks
```

Use `--bench compiler`, `--bench runtime`, `--bench preview` or `--bench lsp` to isolate a target.

## Regression policy

Outside the paired LSP gate above, regression thresholds are explicit review policy. They become
blocking only when the run is measured against an agreed baseline and profile, such as a stable
Linux runner profile or a documented release-measurement profile. Before those baselines exist,
threshold misses are review triggers: investigate the change, record the likely cause, and decide
whether to accept, tune, or follow up.

The
[performance contract](https://github.com/plethu/recite/blob/main/docs/spec/quality.md#198-regression-policy)
owns release review policy. Investigate superlinear behavior and unexplained allocation growth on
representative fixtures as well as timing regressions.

For maintainer profiling workflow, Linux profiler guidance, memory investigation commands, and the
`recite bench` report boundary, see the
[profiling and optimisation playbook](https://github.com/plethu/recite/blob/main/docs/profiling-and-optimisation.md).
