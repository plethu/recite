# Testing, diagnostics and performance

Part of the [production specification](../recite-production-spec.md). These are requirements;
implementation and release readiness require evidence from code, tests and the current GitHub
milestone. Section numbers remain stable.

## 17. Testing

### 17.1 Core Test Philosophy

The project must make dialogue easy to test without a game engine.

Supported test patterns:

- transcript snapshot;
- effect snapshot;
- condition trace snapshot;
- localization fallback snapshot;
- unavailable choice assertion, including no traversal mutation after rejected selection;
- structured availability reason tree snapshot;
- structured presentation projection snapshot;
- hidden-vs-unavailable prompt output assertion;
- blocking effect pause/resume assertion;
- save/load mid-scene assertion;
- save/load while waiting on blocking effect assertion;
- adapter conformance operation/result traces.

### 17.2 Example Rust Test

Use the [runtime integration tests](../../crates/recite-runtime/tests) and
[headless CLI walkthrough](../../docs-site/src/content/docs/examples/headless-cli.md) as executable
examples. Assert emitted values and state transitions, including rejection without mutation;
documentation wording is not a behavioral test.

### 17.3 Fixture Format

The CLI should support a fixture format for headless runs:

```toml
[conditions]
"trust_gte(hazel, rhea, 3)" = true

[choices]
small_talk_start = "7f3a9c2e4b6d8f019a2b"

[effects]
auto_ack_blocking = true

[dialogue]
locale = "fr-FR"

[dialogue.catalogs]
"fr-FR" = ["locale/fr-FR.po"]
fr = ["locale/fr.po"]
```

Condition keys use bare identifiers inside the call, matching the dialogue DSL. The TOML key is
quoted only because TOML requires it for keys containing parentheses; the inner argument list does
not requote identifiers.

The `[dialogue]` fixture table is optional. When present, `locale` selects the runtime dialogue
locale for preview, and `catalogs` maps locale IDs to gettext PO files. Catalog entries use singular
gettext records with `msgctxt` as the stable line or choice ID, `msgid` as source text, and `msgstr`
as translated text. Variant-specific entries may use `id&variant` contexts and should fall back to
`id` before source text. Omitting the table is the valid source-text-only mode; enabling project
dialogue localisation requires the project's declared default and fallback locale/catalog policy.

### 17.4 Adapter Conformance Fixtures

[Adapter contract §13](../engine-adapter-contract.md#13-adapter-conformance-fixtures) owns required
scenarios, capability gates and category drift checks. The published
[fixtures](../../fixtures/adapter-conformance/v1/) supply the operation/result contract; each
adapter must exercise its applicable cases through the real host surface.

### 17.5 Enforced Rust test assurance

`just coverage` uses the pinned cargo-llvm-cov and Rust LLVM tools to measure all-feature workspace
tests. Production line coverage must reach 90% separately in core, compiler, runtime and LSP, 95% in
the parser and 80% in the CLI. `just writer coverage` requires 90% separately in the Writer model
and headless UI. The required Rust and Writer CI lanes enforce these floors and retain JSON
evidence. Tests, examples and benchmark harnesses are excluded from the production denominator;
production error paths and platform boundaries remain included. Line coverage does not establish
branch coverage, assertion strength or native accessibility acceptance.

Generated tests use Proptest with shrinking and standard failure persistence. CI runs 256 cases per
property. State and format properties must compare independent expected values or action models;
round-trip equality alone cannot prove that both endpoints preserve the contract. Reproduce a
failure using its persisted seed and keep a focused regression when correcting production behavior.

`just mutants` runs cargo-mutants over the source-position index, LSP freshness fences, document
close transitions, runtime choice selection and restored pending positions selected in `Cargo.toml`
workspace metadata. The preflight requires mutation candidates from every named file, so an empty
scope or stale path fails. `.cargo/mutants.toml` owns runner settings. Every run includes the
unmutated baseline and core, compiler, LSP and runtime consumers. The required Rust lane runs the
complete critical scope even for test-only changes; it fails on surviving mutants, timeouts or a
broken baseline and retains `mutants.out`. Expand this scope when a new invariant needs stronger
assertion evidence. Other mutation experiments can use `cargo mutants --no-config` with explicit
owners and test packages. Do not remove production paths from coverage or suppress mutants merely to
satisfy a threshold.

## 18. Diagnostics

Diagnostics must be stable and testable.

Each diagnostic should include:

- code;
- severity;
- message;
- file;
- line;
- column;
- optional end line/end column;
- optional related spans;
- optional help text.

The core structured diagnostic contract is authoritative for new producers. Each structured record
has an explicit wire version, stable machine code, severity, source span, a canonical kebab-case
presentation ID, deterministic named typed arguments, ordered related presentations, optional
structured help, and optional structured explanation/remediation guidance. Core records are
locale-neutral and do not depend on Fluent; clients resolve presentation IDs at their own
presentation boundary. The legacy `message` field is retained only as an explicitly supplied
deterministic en-US compatibility fallback while producers migrate. A record's version and closed
field shape are compatibility boundaries: unsupported versions, unknown fields, and duplicate named
arguments are rejected rather than silently overwritten.

Diagnostic codes should be namespaced, for example:

- `RECITE_PARSE001`;
- `RECITE_ID001`;
- `RECITE_SCHEMA001`;
- `RECITE_EFFECT001`;
- `RECITE_META001`;
- `RECITE_MARKUP001`;
- `RECITE_PROJECT001`;
- `RECITE_FRESH001`.

## 19. Performance and Benchmarks

Benchmarks must cover authoring, compilation, runtime traversal, localisation and adapter overhead.
Strategy, workbench and release decisions require evidence naming the fixture, runner, build
profile, measurement method and regression policy. A result applies to that profile, not every
machine or OS.

The numeric targets below guide investigation until realistic baselines establish an enforced
budget. A missed target must be reported, but blocks acceptance only where a reviewed policy names
the baseline, runner, threshold and enforcement point. §19.8 identifies the current paired LSP gate.
Changes to budgets require explicit review.

### Authoring refresh layers

The compiler is whole-project for v1. Recite has several distinct refresh layers:

- LSP live refresh: the editor-facing index re-parses edited files, refreshes diagnostics, and
  resolves cross-file references incrementally.
- Watch/build refresh: `recite watch <project-root>` observes source, schema, and project inputs,
  then re-runs deterministic whole-project validation and asset compilation.
- Adapter import refresh: each engine adapter defines how rebuilt compiled assets enter the host
  asset pipeline and what authors do with active sessions.
- Mid-session patch reload: changing the compiled asset underneath an already running session
  without restarting it is a non-v1 feature.

The v1 requirement is a competitive edit/save/rebuild/import/restart authoring loop, not arbitrary
runtime patching of active dialogue sessions.

### 19.1 Benchmark Harness

The [Criterion targets](../../crates/recite-benchmarks/benches/) own maintainer benchmark cases. Use
the [benchmark guide](../../docs-site/src/content/docs/reference/benchmarks.md) for suite and
fixture selection, and [profiling and optimisation](../profiling-and-optimisation.md) for focused
measurements, sampling and allocation evidence.

`recite bench <fixture-or-project>` is the stable product-facing report surface. It should support
CI-readable JSON, Markdown summaries, local baseline comparison, benchmark-group filtering and
fixture-scale selection. It complements the maintainer harness; Criterion output is not a product
format contract.

Reports must show concrete project shape: source files, blocks, lines, choices, effects, conditions,
generated words and relevant byte sizes. Names such as `tiny` or `epic` identify fixture profiles;
they do not establish performance.

### 19.2 Benchmark Fixtures

The repository must include synthetic and realistic fixtures. Synthetic fixtures must be generated
from named scale profiles so compiler, runtime, CLI, LSP, and adapter benchmarks exercise the same
deterministic project shapes.

The benchmark fixture definitions own each profile's exact counts.

Each synthetic profile must define deterministic structural complexity targets:

- conditions on a representative subset of lines and choices, including shared flags, counters, and
  relationship-style state;
- metadata on blocks, lines, choices, and project inputs;
- deferred, immediate, and blocking effects with schema-checked payload shapes;
- localization catalogs and POT extraction pressure proportional to the localizable entry target;
- cross-block references and branching fan-out sufficient to expose reference resolution and choice
  lookup costs;
- stable line and choice IDs that remain deterministic across generator runs.

Synthetic fixture generation must take structured inputs: the scale profile, the deterministic seed,
schema shape configuration, localization configuration, and any runtime fixture configuration needed
for headless traversal. The generator must produce Recite sources, schema files, runtime fixtures,
and a compact deterministic summary containing counts and content hashes. Summary hashes are the
reviewable signal that regenerated large fixtures still match the expected shape without checking
all generated data into git.

Checked-in synthetic fixture policy:

- check in the generator seed and profile configuration for every profile;
- check in the generated tiny fixture so smoke tests and examples work without a generation step;
- check in compact deterministic summaries for small, medium, large, and epic;
- generate small, medium, large, and epic fixture data on demand for benchmarks, stress checks, and
  profiling runs;
- do not check in generated fixture data whose size would make ordinary source review or clone time
  materially worse.

Realistic fixtures:

- conversation-heavy branching scene;
- object interaction scene set;
- relationship scene set with many conditions;
- localization-heavy scene set;
- effect-heavy scene set with deferred, immediate, and blocking effects.

Realistic fixtures should be compact enough to review by hand and checked into the repository when
possible. Larger realistic fixtures may follow the generated fixture policy when they are derived
from public, MIT OR Apache-2.0-compatible source material or fully synthetic project descriptions.

Measurement hygiene:

- portable suites on Windows, macOS, and Linux must prove generator determinism, CLI stress
  correctness, and benchmark buildability;
- Criterion or the equivalent primary timing harness should provide warmup, sampling, outlier
  handling, baseline comparison, and noise reporting;
- authoritative trend numbers should come from one stable Linux runner or documented local Linux
  profile, not mixed operating-system timing;
- instruction, cache, and heap profiles may use Linux-only external tooling such as Valgrind or
  `perf`, but GPL tooling must remain documented external tooling rather than linked or vendored
  project dependencies;
- benchmark and profiling crate dependencies must be compatible with Recite's MIT OR Apache-2.0
  distribution policy before they are added to the workspace.

### 19.3 Compiler Benchmarks

The [compiler target](../../crates/recite-benchmarks/benches/compiler.rs) owns individual cases.
Required coverage spans parsing, lowering, validation, POT extraction and asset encoding. Validation
measurements must expose schema, reference-resolution, ID-check and markup costs; report compiled
size and the parser/AST memory costs in §19.7.

Initial compile targets on a typical developer laptop are under 100 ms for small projects, 1 s for
medium and 5 s for large. ID checks, reference resolution and schema validation must avoid
superlinear blowups.

### 19.4 Runtime Benchmarks

The [runtime target](../../crates/recite-benchmarks/benches/runtime.rs) owns individual cases.
Coverage must include session start, line/prompt advancement, choice selection, condition dispatch,
all effect modes and blocking acknowledgement, locale lookup, snapshot save/restore and complete
fixture-driven traversal.

After asset load, advancement without condition evaluation should be allocation-free or nearly so;
release line/prompt advancement targets under 50 us per event. Choice selection must be O(1) or
O(log n), never linear over all project choices. Save/load cost should follow session state rather
than compiled asset size, and traversal must not clone full compiled assets.

### 19.5 LSP and Editor Benchmarks

The [LSP target](../../crates/recite-benchmarks/benches/lsp.rs) and
[process harness](../../scripts/lsp_tools/) cover different boundaries. Evidence must include
initial indexing, open/edit parsing, diagnostic refresh, completion, definition, block rename and
index memory. Initial targets are completion under 50 ms for small/medium projects and diagnostics
under 100 ms for typical single-file edits. Large-project indexing should be incremental and
cancellable; a file-local edit must not force unnecessary whole-project parsing.

Measure Writer on every declared platform with the same representative fixture: cold/warm startup,
project open/index, edit-to-diagnostics, schema/catalogue refresh, preview, graph navigation and
idle/active memory. Headless LSP measurements do not establish those native GUI costs.

### 19.6 Engine Adapter Benchmarks

Measure asset loading/conversion, event and typed-wrapper conversion, active-session update, and
host condition dispatch. Package guides own host-specific probes. Idle adapters should add
negligible frame cost.

### 19.7 Memory Metrics

Report parser tree size, AST allocation volume, compiled asset size, peak compiler memory, session
size, LSP index size, hot-path allocations and large string/metadata clones where practical. The
[profiling guide](../profiling-and-optimisation.md) distinguishes retained memory, allocation churn
and structural size reports. Runtime data should remain shared and immutable, with compact session
state.

### 19.8 Regression Policy

Pull requests affecting core Rust, fixtures or shared build inputs, and the weekly complete run,
should execute the existing tiny compiler/runtime/preview/LSP Criterion smoke targets. Documentation
changes do not require benchmark builds. Smoke proves execution; fuller suites belong on release
branches or scheduled jobs.

The paired LSP gate compares base and candidate binaries on the same named Linux runner. Its
[policy](../../scripts/lsp-performance-policy.json) and [harness](../../scripts/lsp_tools/) own
workloads, samples, thresholds and confirmation rounds. It requires complete coverage and matching
output fingerprints, with raw samples and binary identities retained as CI artifacts. It covers
those process workloads, not cross-platform rendering or long-lived memory.

Outside that gate, initial review triggers are more than 10% regression in hot runtime paths, more
than 20% in compiler/LSP paths, accidental O(n^2) work on medium/large fixtures, or unexplained
hot-path allocation growth. GUI evidence must also record blocking accessibility regressions such as
lost focus, screen-reader dead ends, unusable zoom or inaccessible high-contrast controls.

### 19.9 Trace Metrics

`recite trace --metrics` provides optional instrumentation; timing fields are not snapshot-stable.
Default traces remain deterministic. The
[CLI trace reference](../../docs-site/src/content/docs/reference/cli.md) owns invocation, and
[`TraceMetrics`](../../crates/recite-cli/src/runtime_fixture/trace/model.rs) owns the report fields.
