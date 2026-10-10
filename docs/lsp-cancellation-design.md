# LSP architecture and performance decisions

The server accepts editor changes in order, analyses immutable snapshots off the protocol loop, and
checks freshness and cancellation through writer handoff. This is the current maintainer overview.
The [tooling contract](spec/tooling.md#14-lsp) and
[performance policy](spec/quality.md#195-lsp-and-editor-benchmarks) define requirements;
measurements establish only the workloads and platforms they actually cover.

## Ownership and change paths

| Concern                       | Owner                                                                            | Contract                                                                                                                     |
| ----------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Framing and stdio transport   | `lsp-server`                                                                     | Recite does not maintain a second wire transport                                                                             |
| Accepted open-document text   | `crates/recite-lsp/src/server/text_sync.rs`                                      | Strict UTF-16 positions, atomic edit batches and monotonic versions before coalescing                                        |
| Scheduling and publication    | `server.rs`, `server/scheduling.rs`, `server/requests.rs`, `server/freshness.rs` | Bounded work, ordered barriers, cancellation and snapshot fences through delivery                                            |
| Background execution          | `server/workers.rs`                                                              | Analysis and queries have separate workers; worker input uses bounded standard channels and selectable results use Crossbeam |
| Project and query snapshots   | `workspace/`                                                                     | Share unchanged immutable analyses; retain project authority for operations such as rename                                   |
| Incremental semantic analysis | `recite-compiler` authoring and validation modules                               | Reuse unchanged regions, relocate final output spans and rebuild when dependencies require it                                |
| External measurements         | `scripts/lsp_tools/`, invoked through `just perf`                                | Separate process latency, resource accounting and report validation from production server code                              |

A text or protocol acceptance change starts at `text_sync.rs` and the stdio transaction tests.
Scheduling changes start at the coordinator and final-handoff cancellation tests. Semantic
invalidation changes start in the compiler, with fresh/incremental differential evidence. Tests
preserve diagnostic and query outputs, stable IDs, source spans and prior snapshots.

## Retained decisions

The [dependency record](lsp-dependency-decisions.md) owns transport/framework/text-library choices,
rejection reasons and reopening conditions. Incremental acceptance and immutable snapshots retain
strict edit semantics; compiler invalidation remains shared with other authoring surfaces.

## Normal commands

Use the [profiling guide](profiling-and-optimisation.md) for maintained commands, workload selection
and instrumentation boundaries. `just check` is the complete local gate.

## Regression protection and remaining limits

[Quality §19.8](spec/quality.md#198-regression-policy) owns budgets and enforcement. Results apply
to their named workloads and profiles; they do not establish arbitrary-session leak freedom or
cross-platform latency. Further optimisation needs an attributed practical workload and measured
maintenance or performance benefit. Distinguish live heap, allocation churn and process RSS before
choosing a fix.

## Evidence and concluded experiments

Concluded reports are available at `6e32b614` through Git history; the
[final resource investigation](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/final-resource-profiling.md)
contains the capacity-change reproduction. Retired diagnostic tools and workflow inputs are
preserved at `1004de99594d`. Export the named revision when reproducing an old command; do not
restore settled experiments to normal CI. Current dependency decisions and the profiling playbook
own new investigations.
