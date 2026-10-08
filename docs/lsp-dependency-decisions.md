# Current LSP dependency decisions

This is the maintained decision owner. The
[October 2026 assessment](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/dependency-decisions.md)
preserves candidate versions, maintainer/licensing sources, bounded controls and raw evidence.
Results belong to those revisions and profiles, not every future release. Requirements remain in the
[tooling contract](spec/tooling.md#14-lsp).

## Retained choices

| Candidate                      | Decision                                                 | Evidence and reopening condition                                                                                                                                                                                                                                                                                                                  |
| ------------------------------ | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `lsp-server` 0.7.9             | Keep                                                     | Already owns framing and stdio transport. Its rust-analyzer home has continuing maintenance. Revisit for a reproduced component defect or concrete missing capability.                                                                                                                                                                            |
| `memchr` 2.8                   | Keep for source-line scanning                            | Established MIT/Unlicense byte-search implementation, already present in both locked workspaces. Use its optimized two-byte search for LF/CR boundaries; keep authored-byte and line semantics in the shared iterator. Revisit if standard-library search removes the measured latency cost. [Upstream](https://github.com/BurntSushi/memchr).    |
| `gen-lsp-types` 0.11.0         | Preferred future types replacement; defer this migration | Maintained generated protocol coverage and real rust-analyzer/Ruff adoption are positive. Current edits require a strict ingress representation; no needed new editor capability or net maintenance reduction was demonstrated. Revisit for an upstream edit-union repair, a needed protocol feature, or a dedicated protocol modernization task. |
| `tower-lsp-server` 0.24.0-rc.1 | Defer framework integration                              | Async notification handlers may overlap; concurrency one disables effective cancellation. Cancellation ends at handler completion, before writer delivery. Preserving Recite's stronger rules retains its coordinator and adds integration machinery. Revisit only with a supported integration boundary that removes material ownership.         |
| `async-lsp` 0.2.4              | Defer                                                    | Released main-loop/concurrency composition stalls at saturation even after pending work becomes ready. A capacity-two control completes. Revisit after a released repair and a credible deletion inventory; do not assume ownership of a fork.                                                                                                    |
| `lsp-textdocument` 0.5.0       | Reject for this edit boundary                            | Surrogate-interior positions round down; a failed batch can mutate earlier edits before panicking. Recite must retain strict validation, transaction staging, stale-version checks and snapshot normalization. This replaces little of the owned complexity. Revisit if an API directly supplies those contracts.                                 |
| Salsa                          | Defer                                                    | Reputable incremental-analysis dependency, but it changes semantic analysis and invalidation ownership. No bounded query or maintenance bottleneck currently earns that migration.                                                                                                                                                                |

The additional text/framework assessment retained String and the current coordinator. `str_indices`
is the preferred narrow utility if a relevant hotspot earns it; `line-index`, Ropey, crop and lspf
did not demonstrate sufficient net benefit for the current contracts. The
[candidate experiments](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/dependency-decisions.md#additional-text-and-framework-spikes)
record positive controls as well as rejection reasons. No disposable prototype is a maintained
second implementation.

## Reevaluation plan

At the v1 dependency review, refresh release, maintainer and advisory information for current
dependencies and `gen-lsp-types`. Repeat that lightweight review when planning a protocol upgrade or
encountering a dependency defect. A newer version or an elapsed date alone does not justify
repeating the entire experiment suite. There is no scheduled benchmark job or automatic migration
attached to this plan.

Reopen a dependency for a required capability, a repaired contract violation or a measured ownership
problem. For text libraries, replay strict UTF-16/CR/CRLF/LF acceptance, atomic edits and stale
versions, including construction, flattening and retained memory. For frameworks, test saturated
cancellation, ordered mutation, shutdown and publication freshness. For Salsa, replace one bounded
analysis query before considering a whole-analysis migration.

For every reopened candidate, refresh maintenance, licensing, supported features and platform
requirements before compiling it. Use the retained reproductions as a starting point and the
then-current production revision as the new control. Accept a dependency for a substantial net
reduction in owned complexity or a consistent relevant performance/capability gain. A roughly 5%
slowdown is a possible maintenance trade, not permission to weaken correctness or ignore a supported
workload. Name any unmeasured surfaces, including long-session resources and macOS/Windows, before
making adoption claims.

Update this record when a decision changes. Keep old measurements tied to their versions and
execution profile; record the new revision and superseding evidence instead of silently treating the
October 2026 results as current.

## Maintainer tooling language

The Python LSP harness is retained provisionally as an already validated measurement owner. Its
existing code is not evidence that Python is the best default for new tooling. Count maintainer
learning, setup, debugging, runtime interference, dependency footprint and the cost of replacing the
whole owner. Cross-platform counters alone do not distinguish Python from Go.

The
[bounded language probes](https://github.com/plethu/recite/blob/6e32b614bd8c91a6616f02ec2991b7e300808129/docs/archive/lsp-optimisation/tooling-language-evidence.json.gz)
record dependency locks and tested process/protocol seams, not complete ports or comparative
end-to-end performance.

| Option                         | Benefit and remaining cost                                                                                                                                                                                                                                                                                                                                               |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Rust, private maintainer crate | Existing language, toolchain and quality gates; preferred starting point for substantial new repository tooling. `sysinfo` supplies most counters, but `tasks()` remains Linux-only. Safe libproc and Windows adapters are plausible additions whose ownership and native behavior still need verification. Keep these dependencies out of shipped CLI/benchmark crates. |
| Go + gopsutil v4.26.9          | Strongest extra-language challenger for the external harness. Native implementations supply CPU/RSS/threads and Unix descriptors or Windows handles on all three target OSes without a custom Recite adapter. Adds Go 1.26, modules and platform dependencies; replacing Python means porting workload/report/gate logic too.                                            |
| Python + psutil                | Existing validated harness and native counter library. Adds Python/uv, environment management and learning cost; use one locked package/CLI. Retention is bounded to this owner, not permission to grow Python across unrelated automation.                                                                                                                              |
| Node/TypeScript                | Existing editor protocol ecosystem works, but inspected process packages do not supply the complete resource contract. No case demonstrated for expanding Node or splitting the harness across runtimes.                                                                                                                                                                 |

Native alternative runs on macOS and Windows remain unperformed. Invalid or exited-process
measurements must fail closed, including CPU-only idle samples.

For the next substantial harness ownership change, compare a private Rust tool and Go against the
existing Python owner before extending it. A full replacement must preserve report schemas,
growth-fault detection, lifecycle cleanup and the three-OS gate, and demonstrate reduced total
maintenance or material interference reduction. Measure cold setup and warm commands as well as
driver CPU/RSS. Preserve the complete-body timestamp before JSON decoding, encoding before the send
timer, and monotonic elapsed time; recalibrate budgets explicitly if those boundaries change. Do not
drop a counter or add a permanent sampler daemon merely to make a language migration fit.
