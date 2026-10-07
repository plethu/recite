# Current LSP dependency decisions

This is the maintained decision owner. The [October 2026 assessment](archive/lsp-optimisation/dependency-decisions.md)
preserves candidate versions, maintainer/licensing sources, bounded controls and
raw evidence. Results belong to those revisions and profiles, not every future
release. Requirements remain in the [tooling contract](spec/tooling.md#14-lsp).

## Retained choices

| Candidate | Decision | Evidence and reopening condition |
| --- | --- | --- |
| `lsp-server` 0.7.9 | Keep | Already owns framing and stdio transport. Its rust-analyzer home has continuing maintenance. Revisit for a reproduced component defect or concrete missing capability. |
| `gen-lsp-types` 0.11.0 | Preferred future types replacement; defer this migration | Maintained generated protocol coverage and real rust-analyzer/Ruff adoption are positive. Current edits require a strict ingress representation; no needed new editor capability or net maintenance reduction was demonstrated. Revisit for an upstream edit-union repair, a needed protocol feature, or a dedicated protocol modernization task. |
| `tower-lsp-server` 0.24.0-rc.1 | Defer framework integration | Async notification handlers may overlap; concurrency one disables effective cancellation. Cancellation ends at handler completion, before writer delivery. Preserving Recite's stronger rules retains its coordinator and adds integration machinery. Revisit only with a supported integration boundary that removes material ownership. |
| `async-lsp` 0.2.4 | Defer | Released main-loop/concurrency composition stalls at saturation even after pending work becomes ready. A capacity-two control completes. Revisit after a released repair and a credible deletion inventory; do not assume ownership of a fork. |
| `lsp-textdocument` 0.5.0 | Reject for this edit boundary | Surrogate-interior positions round down; a failed batch can mutate earlier edits before panicking. Recite must retain strict validation, transaction staging, stale-version checks and snapshot normalization. This replaces little of the owned complexity. Revisit if an API directly supplies those contracts. |
| Salsa | Defer | Reputable incremental-analysis dependency, but it changes semantic analysis and invalidation ownership. No bounded query or maintenance bottleneck currently earns that migration. |

The additional text/framework assessment retained String and the current
coordinator. `str_indices` is the preferred narrow utility if a relevant hotspot
earns it; `line-index`, Ropey, crop and lspf did not demonstrate sufficient net
benefit for the current contracts. The [candidate experiments](archive/lsp-optimisation/dependency-decisions.md#additional-text-and-framework-spikes)
record positive controls as well as rejection reasons. No disposable prototype
is a maintained second implementation.

## Reevaluation plan

At the v1 dependency review, refresh release, maintainer and advisory information
for current dependencies and `gen-lsp-types`. Repeat that lightweight review when
planning a protocol upgrade or encountering a dependency defect. A newer version
or an elapsed date alone does not justify repeating the entire experiment suite.
There is no scheduled benchmark job or automatic migration attached to this plan.

| Candidate | When to reopen | First experiment and evidence needed |
| --- | --- | --- |
| `gen-lsp-types` | First priority at a needed protocol-feature upgrade, a released edit-union repair, or a dedicated replacement of frozen `lsp-types`. Check its status during the v1 dependency review. | Rerun malformed-edit decoding and missing/null controls. Preserve strict acceptance through a small ingress representation if necessary; then establish whole-server wire parity. The existing compile-error count is not a rejection criterion. |
| `str_indices` | A profile shows scalar/UTF-16 counting or conversion materially contributes to diagnostics/query latency, or those loops become a demonstrated maintenance problem. | Try a narrow count/conversion replacement before reviving the combined prototype. Measure the affected operation end to end; this narrower variant is currently untested. |
| Ropey | Real editing profiles show transaction copying or offset lookup dominates, representative documents grow beyond the tested sizes, or an independently justified analysis change can consume chunks. | Reapply the protocol-store prototype. Include initial construction, repeated edits, full replacements, flattening, peak memory and Unicode-feature unification. Improve the relevant workload without hiding the previous full-replacement regressions. |
| `line-index` | Repeated LF-based compiler/query conversions become hot and a snapshot can reuse one index across enough operations to amortize construction. | Compare the complete projection path, including construction and retained memory. Preserve scalar columns, source spans and CRLF treatment. Protocol bare-CR support needs its own explicit solution. |
| crop | A released API supports the required CR/CRLF/LF semantics, or a separate chunk-based text-buffer requirement makes its byte-oriented representation useful. | Rerun the complete position corpus, strict surrogate checks and atomic transactions; compare against Ropey as well as String, including flattening. Avoid maintaining a second line index merely to adapt it. |
| lspf | A release offers pre-mutation validation/rejection or custom document ownership, plus a supported publication boundary that can preserve freshness and cancellation. Alternatively, a demonstrated coordinator maintenance burden justifies testing its existing interception APIs. | Rerun the built-server controls and all rejected-input cases. Count the transport interceptor, validation state, encoding adaptation and publication coordination against gross deletions. Record the required toolchain upgrade; a 1.x label alone does not establish fit. |
| Tower community fork | A supported integration hook preserves ordered mutation and the required publication authority, or lifecycle/routing code grows into a material maintenance burden. | Test suspended notification ordering, saturated cancellation and cancellation before writer acceptance. Inventory actual removed owners; a stable release alone does not establish savings. |
| `async-lsp` | A released version repairs saturation, and there is a concrete reason to replace protocol orchestration. | Rerun the explicit readiness/gate probe with its capacity-two control before any backend migration. Then test ordering, shutdown and final publication authority. |
| `lsp-textdocument` | A released transactional/checked API replaces substantial strict edit validation rather than requiring it alongside the dependency. | Replay malformed batches, stale versions, surrogate interiors, `rangeLength` and all newline conventions. Count remaining validation and staging code before benchmarking. |
| Salsa | Profiling or concrete maintenance changes show manual query invalidation/recomputation is a bottleneck; evidence points beyond protocol transport. | Replace one bounded analysis query and compare invalidation ownership, deterministic diagnostics, cancellation, recovery and retained memory. Do not begin with a whole-analysis rewrite. |

For every reopened candidate, refresh maintenance, licensing, supported features
and platform requirements before compiling it. Use the retained reproductions
as a starting point and the then-current production revision as the new control.
Accept a dependency for a substantial net reduction in owned complexity or a
consistent relevant performance/capability gain. A roughly 5% slowdown is a
possible maintenance trade, not permission to weaken correctness or ignore a
supported workload. Name any unmeasured surfaces, including long-session
resources and macOS/Windows, before making adoption claims.

Update this record when a decision changes. Keep old measurements tied to their
versions and execution profile; record the new revision and superseding evidence
instead of silently treating the October 2026 results as current.

## Maintainer tooling language

The Python LSP harness is retained provisionally as an already validated
measurement owner. Its existing code is not evidence that Python is the best
default for new tooling. Count maintainer learning, setup, debugging, runtime
interference, dependency footprint and the cost of replacing the whole owner.
Cross-platform counters alone do not distinguish Python from Go.

The [bounded language probes](archive/lsp-optimisation/tooling-language-evidence.json.gz)
preserve source, dependency locks and Linux observations. Rust/sysinfo and
Go/gopsutil matched deliberate memory/CPU/thread/descriptor growth. Node's
existing JSON-RPC client and the Go client matched initialize, open/edit
diagnostics and definition against the Python control. Go also sampled the live
server, shut it down and rejected reads from the reaped child. These are critical
seams, not complete ports or comparative end-to-end performance measurements.

| Option | Benefit and remaining cost |
| --- | --- |
| Rust, private maintainer crate | Existing language, toolchain and quality gates; preferred starting point for substantial new repository tooling. `sysinfo` supplies most counters, but `tasks()` remains Linux-only. Safe libproc and Windows adapters are plausible additions whose ownership and native behavior still need verification. Keep these dependencies out of shipped CLI/benchmark crates. |
| Go + gopsutil v4.26.9 | Strongest extra-language challenger for the external harness. Native implementations supply CPU/RSS/threads and Unix descriptors or Windows handles on all three target OSes without a custom Recite adapter. Adds Go 1.26, modules and platform dependencies; replacing Python means porting workload/report/gate logic too. |
| Python + psutil | Existing validated harness and native counter library. Adds Python/uv, environment management and learning cost; use one locked package/CLI. Retention is bounded to this owner, not permission to grow Python across unrelated automation. |
| Node/TypeScript | Existing editor protocol ecosystem works, but inspected process packages do not supply the complete resource contract. No case demonstrated for expanding Node or splitting the harness across runtimes. |

Go and psutil took approximately 34 and 17 microseconds per settled Linux
sample respectively (six alternating rounds of 1,000 samples, excluding startup
and output encoding). Neither establishes material checkpoint interference or
whole-driver superiority. The Go sampler cross-compiled for macOS ARM64 and
Windows x86_64 with CGo disabled; native alternative runs remain unperformed.
The inspected macOS gopsutil methods can ignore failed native reads. Invalid or
exited-process samples must fail closed, including CPU-only idle measurements.

For the next substantial harness ownership change, compare a private Rust tool
and Go against the existing Python owner before extending it. A full replacement
must preserve report schemas, growth-fault detection, lifecycle cleanup and the
three-OS gate, and demonstrate reduced total maintenance or material interference
reduction. Measure cold setup and warm commands as well as driver CPU/RSS.
Preserve the complete-body timestamp before JSON decoding, encoding before the
send timer, and monotonic elapsed time; recalibrate budgets explicitly if those
boundaries change. Do not drop a counter or add a permanent sampler daemon merely
to make a language migration fit.

## Comparable Rust projects

Primary-source inspection on 7 October 2026 found deliberate mixtures rather
than a universal Rust-project convention:

- [rust-analyzer's Rust xtask](https://github.com/rust-lang/rust-analyzer/blob/master/xtask/src/main.rs)
  owns auxiliary build, installation, distribution, code generation and metrics.
- [Bevy's Rust CI tool](https://github.com/bevyengine/bevy/blob/main/tools/ci/src/main.rs)
  owns its command dispatcher in an unpublished tools workspace member.
- [Rust bootstrap](https://github.com/rust-lang/rust/blob/main/src/bootstrap/README.md)
  uses Python to acquire the stage-zero toolchain and compile the main Rust build
  system. Its Python entrypoint is not evidence that the whole build system is Python.
- [uv's contributor guide](https://github.com/astral-sh/uv/blob/main/CONTRIBUTING.md)
  documents Rust development/generation, Python external benchmark scripts and
  additional formatting/docs tools. Its Python-product context differs from Recite.

The inference for Recite is to reuse its Rust ownership for substantial general
tooling, evaluate a specialist external harness separately, and keep one ordinary
command owner. None of these examples establishes that adopting their entire
toolchain mix would improve this repository.

References: [psutil process API](https://psutil.io/api/),
[sysinfo tasks](https://docs.rs/sysinfo/latest/sysinfo/struct.Process.html#method.tasks),
[libproc task API](https://docs.rs/libproc/0.14.11/libproc/proc_pid/fn.pidinfo.html),
[gopsutil Windows counters](https://github.com/shirou/gopsutil/blob/v4.26.9/process/process_windows.go),
[macOS counters](https://github.com/shirou/gopsutil/blob/v4.26.9/process/process_darwin.go),
[Go module footprint](https://github.com/shirou/gopsutil/blob/v4.26.9/go.mod),
[Go monotonic clocks](https://pkg.go.dev/time#hdr-Monotonic_Clocks),
[pidusage](https://github.com/soyuka/pidusage),
[systeminformation process fields](https://systeminformation.io/processes.html).
