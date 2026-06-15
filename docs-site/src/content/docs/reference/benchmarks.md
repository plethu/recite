---
title: Benchmarks
description: Commands and policy for Recite benchmark smoke checks and regression review.
---

Recite benchmarks live in `crates/recite-benchmarks` and use Criterion against
the shared synthetic fixture profiles and checked-in realistic fixture packs.
The suite is split into explicit compiler, runtime, and LSP bench targets.

## Fast smoke

Use the smoke script for pull-request and CI checks that need to prove the
benchmark targets still build and execute quickly:

```bash
scripts/benchmark-smoke.sh
```

The script runs only the checked-in tiny fixture data and never asks the fixture
generator for larger profiles:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench compiler -- 'compiler/.*/tiny' --test
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime -- 'runtime/.*/tiny' --test
```

Criterion `--test` mode executes each matching benchmark once. This is a
non-comparative smoke check; it does not save baselines, compare timings, or
enforce regression thresholds.

## Full suites

Run all bench targets with the local default scale set:

```bash
cargo bench -p recite-benchmarks
```

Without `RECITE_BENCH_SCALES`, the benchmark crate runs `tiny,small`. Select
heavier generated profiles explicitly when release review or profiling needs
them:

```bash
RECITE_BENCH_SCALES=medium cargo bench -p recite-benchmarks
RECITE_BENCH_SCALES=large,epic cargo bench -p recite-benchmarks -- --sample-size 10
```

The same selector also accepts checked-in realistic packs:

```bash
RECITE_BENCH_SCALES=realistic:v1-pack cargo bench -p recite-benchmarks
RECITE_BENCH_SCALES=tiny,realistic:v1-pack cargo bench -p recite-benchmarks
```

Target one side of the suite when isolating a change:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp
```

## Regression policy

Regression thresholds are explicit review policy. They become blocking only when
the run is measured against an agreed baseline and profile, such as a stable
Linux runner profile or a documented release-measurement profile. Before those
baselines exist, threshold misses are review triggers: investigate the change,
record the likely cause, and decide whether to accept, tune, or follow up.

Use these starting thresholds:

- more than 10% regression in hot runtime paths;
- more than 20% regression in compiler and LSP paths;
- any accidental superlinear behavior on medium or large fixtures;
- unexpected allocation increases in allocation-sensitive runtime benchmarks.

Hot runtime paths include `start_scene`, `next` for line and prompt events,
choice selection, condition dispatch, effect emission and acknowledgement,
locale lookup, session encode/decode, and full traversal. Compiler and LSP paths
include parsing, lowering, validation, schema validation, compilation, POT
extraction, project indexing, open-file parse, diagnostics refresh, completion,
and go-to-definition.

For maintainer profiling workflow, Linux profiler guidance, memory investigation
commands, and the planned `recite bench` mapping, see the
[profiling and optimisation playbook](https://codeberg.org/plethu/recite/src/branch/main/docs/profiling-and-optimisation.md).
