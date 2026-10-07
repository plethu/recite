# LSP architecture and performance decisions

The server accepts editor changes in order, analyses immutable snapshots off the
protocol loop, and checks freshness and cancellation through writer handoff.
This is the current maintainer overview for [#206](https://github.com/plethu/recite/issues/206).
The [tooling contract](spec/tooling.md#14-lsp) and
[performance policy](spec/quality.md#195-lsp-and-editor-benchmarks) define requirements;
measurements establish only the workloads and platforms they actually cover.

## Ownership and change paths

| Concern | Owner | Contract |
| --- | --- | --- |
| Framing and stdio transport | `lsp-server` | Recite does not maintain a second wire transport |
| Accepted open-document text | `crates/recite-lsp/src/server/text_sync.rs` | Strict UTF-16 positions, atomic edit batches and monotonic versions before coalescing |
| Scheduling and publication | `server.rs`, `server/scheduling.rs`, `server/requests.rs`, `server/freshness.rs` | Bounded work, ordered barriers, cancellation and snapshot fences through delivery |
| Background execution | `server/workers.rs` | Analysis and queries have separate workers; worker input uses bounded standard channels and selectable results use Crossbeam |
| Project and query snapshots | `workspace/` | Share unchanged immutable analyses; retain project authority for operations such as rename |
| Incremental semantic analysis | `recite-compiler` authoring and validation modules | Reuse unchanged regions, relocate final output spans and rebuild when dependencies require it |
| External measurements | `scripts/lsp_tools/`, invoked through `just perf` | Separate process latency, resource accounting and report validation from production server code |

A text or protocol acceptance change starts at `text_sync.rs` and the stdio
transaction tests. Scheduling changes start at the coordinator and final-handoff
cancellation tests. Semantic invalidation changes start in the compiler, with
fresh/incremental differential evidence. Tests preserve diagnostic and query
outputs, stable IDs, source spans and prior snapshots.

## Retained decisions

- Incremental sync avoids editor-side full-text batching while retaining strict
  transactional text acceptance.
- Exact saved-schema identity, unchanged region reuse and final-array span
  relocation reduce repeated work without changing semantic authority.
- Parked bounded worker inputs and moving completed JSON through response
  handoff simplify ownership and reduce measured scheduling/allocation costs.
- Region output composition and completion reserve useful known capacities;
  these are small changes with direct allocation evidence.
- Existing framing, coordinator and freshness ownership remain. Framework and
  text-library alternatives did not demonstrate enough net complexity reduction
  while preserving the required contracts.

The [dependency decision record](lsp-dependency-decisions.md)
contains candidate versions, maintenance assessment, bounded probes and exact
reopening conditions. A roughly 5% slowdown can be worth substantial maintenance
reduction; the decision counts adapters and retained validation, not gross file
deletions. Reopen a rejected candidate when its named condition changes.

## Normal commands

```sh
just perf setup
just perf check
just perf bench lsp large,realistic:v1-pack
just perf compare BASE_COMMIT
just perf build
just perf sessions --binary target/release/recite-lsp --output target/lsp-sessions
just perf lsp endurance --binary target/release/recite-lsp --output target/lsp-session.json
just perf lsp --help
```

Use `recite-lsp.exe` on Windows. `compare` builds both revisions before sampling;
its scratch builds need an outside-checkout `TMPDIR` with enough disk space.
The [profiling playbook](profiling-and-optimisation.md) covers discovery,
instrumentation boundaries and completion evidence. `just check` is the complete
local gate; `just maintainability` runs focused structural checks.

## Regression protection and remaining limits

CI compares release servers in alternating pairs and rejects confirmed material
regressions with matching fixture/output identities. The Linux comparison covers
edits, queries, indexing, opening and peak RSS. The session workflow retains
fixed-set and document-churn probes, fault injection, recovery/idle CPU budgets
and rendered editor checks on Linux, Windows and macOS. Budgets and evidence
boundaries live in [§19.8](spec/quality.md#198-regression-policy), not this overview.

Those gates do not establish universal latency, arbitrary-session leak freedom
or a best-in-class comparison. The final profile still attributes work to text
position conversion, dependency lookups, AST construction and large response
serialization. Further changes need a practical workload and a measured
maintenance/performance benefit, as described in the
[final resource investigation](archive/lsp-optimisation/final-resource-profiling.md#stopping-and-future-investigations).

## Evidence and concluded experiments

Start with the dependency decisions and final resource investigation above.
The [resource tradeoff](archive/lsp-optimisation/resource-tradeoff.md) records the
retained worker/JSON decisions. The former chronological overview is preserved
as [historical implementation evidence](archive/lsp-optimisation/history.md).

Completed channel, driver-accounting, stopped-response and macOS yield probes
are [retired experiments](archive/lsp-optimisation/retired-probes.md). Their named
revision preserves the tools and workflow needed to reproduce them. They are
not permanent CI modes or another maintained implementation.
