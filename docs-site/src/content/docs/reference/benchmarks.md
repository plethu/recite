---
title: Benchmarks
description: Commands and policy for Recite benchmark smoke checks and regression review.
---

Recite benchmarks live in `crates/recite-benchmarks` and use Criterion against
the shared synthetic fixture profiles and checked-in realistic fixture packs.
The suite is split into explicit compiler, runtime, preview, and LSP bench
targets.

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
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview -- 'preview/.*/tiny' --test
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp -- 'lsp/.*/tiny' --test
```

Criterion `--test` mode executes each matching benchmark once. This is a
non-comparative smoke check; it does not save baselines, compare timings, or
enforce regression thresholds.

The CI benchmark lane also runs a separate paired LSP regression check:

```bash
mise exec -- bash scripts/check-lsp-performance.sh BASE_COMMIT
```

This builds both revisions before timing them on the same runner. The 24
comparisons cover six edit workloads in full and negotiated sync modes, eight
query/action workloads, shared-destination invalidation, fresh-process indexing
and opening, and process peak RSS. The shared-destination fixture has 100 files,
2,000 blocks and 20,000 dialogue lines, with references to ten shared files.
Each metric has 21 observations after two warmups in three alternating pairs.

Both relative and absolute increases must exceed the policy in at least two
pairs and recur in a second round:

| Surface | Relative increase | Absolute increase |
| --- | ---: | ---: |
| Warm edits and queries | 20% | 2 ms |
| Fresh-process readiness | 30% | 50 ms |
| Open document | 30% | 10 ms |
| Peak RSS through indexing/open | 20% | 16 MiB |

Output fingerprints and fixture identities must match. Missing or inconsistent
evidence fails the check. Raw samples and binary/revision identities are saved
under `target/lsp-performance/` and uploaded by CI. Fresh-process samples use a
warm filesystem; they are not cold-storage measurements.

Full and ranged sustained-session probes check cancellation and continued use.
A burst probe sends 25 edits at 5 ms intervals, then permits a 150 ms pause.
Diagnostics and successful completion, definition, rename and fix-all responses
must recover within 500 ms of the last edit. No stale result is accepted as a
successful response. The comparison runs on Linux and does not establish
cross-platform rendering or long-lived memory budgets.

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

The preview target is the exception: its no-environment default is the checked-in
`tiny` fixture plus `realistic:v1-pack`, so traversal evidence covers both a
generated shape and an authored project.

The same selector also accepts checked-in realistic packs:

```bash
RECITE_BENCH_SCALES=realistic:v1-pack cargo bench -p recite-benchmarks
RECITE_BENCH_SCALES=tiny,realistic:v1-pack cargo bench -p recite-benchmarks
```

Target one side of the suite when isolating a change:

```bash
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench compiler
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench runtime
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench preview
RECITE_BENCH_SCALES=tiny cargo bench -p recite-benchmarks --bench lsp
```

## Regression policy

Outside the paired LSP gate above, regression thresholds are explicit review policy. They become blocking only when
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
[profiling and optimisation playbook](https://github.com/plethu/recite/blob/main/docs/profiling-and-optimisation.md).
