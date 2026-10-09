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

```rust
let asset = compile_fixture("small_talk.recite");
let mut session = start_scene(&asset, Some("small_talk_start"), locale!("en-GB"))?;
let mut fixture = DialogueFixture::default()
    .with_condition("trust_gte(hazel, rhea, 3)", true)
    .auto_ack_effects();

let trace = run_to_end(&mut session, &fixture)?;

assert_snapshot!(trace.transcript);
assert_eq!(
    trace.deferred_effects,
    vec![
        effect!("advance_thread", "rhea_job_response", "fine"),
    ],
);
```

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

The normative adapter conformance fixture contract is in `docs/engine-adapter-contract.md` §13 and
is backed by `fixtures/adapter-conformance/v1/`.

Testing policy:

- mandatory scenarios cover every stable adapter error category from contract §12;
- source/schema freshness scenarios are capability-gated by adapter-declared source/schema import
  visibility;
- compiled-asset compatibility and save/load identity scenarios are mandatory;
- scenarios that require concrete adapters remain in the manifest as `adapter_runner_required` with
  operation/result shape and runner notes;
- reference-driver checks must fail when §12 categories drift from fixture schema tables.

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

Performance is part of the product contract. Recite is intended for games that validate dialogue in
CI, run headless tests frequently, and may load large narrative projects during editor workflows.
Benchmarks must cover authoring, compilation, runtime traversal, localization, and adapter overhead.

All numeric budgets in this section are **aspirational targets, not contracts**, until a baseline
exists from realistic fixtures. They will be re-evaluated against measured numbers; failing to hit a
target is a benchmark report, not an automatic acceptance failure. Evidence is nevertheless required
at the strategy, workbench, and release gates: each release baseline must name its fixture, runner,
build profile, measurement method, and regression policy. Benchmarks are part of the serious-v1
evidence package even where a target is not yet a hard threshold.

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

The workspace must include repeatable benchmarks using Criterion or an equivalent Rust benchmark
framework.

Benchmarks must be runnable through:

```text
cargo bench
recite bench <fixture-or-project>
```

`cargo bench` is the maintainer microbenchmark harness. It may use Criterion warmup, sampling,
plots, local profiler integration, and maintainer-only target selection without treating its output
format as a product contract.

`recite bench` is the stable product-facing report surface. It exists for adoption evidence, release
notes, CI-readable JSON, and local comparison against an explicitly supplied baseline snapshot. It
must not replace or weaken the maintainer benchmark harness.

The CLI benchmark command should support:

- JSON output for CI comparison;
- Markdown summary output for release notes;
- baseline comparison against a checked-in, downloaded, or otherwise local benchmark snapshot;
- filtering by benchmark group;
- fixture scale selection.

Synthetic names such as `tiny`, `small`, `medium`, `large`, and `epic` are fixture IDs, not
self-explanatory performance claims. Every user-facing report must include concrete project-shape
counts such as source files, blocks, dialogue lines, choices, effects, conditions, generated words,
and relevant byte sizes where available.

Timing deltas are evidence for the named run profile that produced them. They are not absolute
performance guarantees, cross-machine promises, or hard release gates unless a separate regression
policy explicitly defines a baseline, runner profile, threshold, and enforcement point.

### 19.2 Benchmark Fixtures

The repository must include synthetic and realistic fixtures. Synthetic fixtures must be generated
from named scale profiles so compiler, runtime, CLI, LSP, and adapter benchmarks exercise the same
deterministic project shapes.

Synthetic scale profiles:

| Profile | Blocks |  Lines | Choices | Localizable entries | Generated words |
| ------- | -----: | -----: | ------: | ------------------: | --------------: |
| tiny    |     10 |    100 |      20 |           about 120 |     about 1,000 |
| small   |    100 |  1,000 |     200 |         about 1,200 |    about 10,000 |
| medium  |  1,000 | 10,000 |   2,000 |        about 12,000 |   about 100,000 |
| large   |  5,000 | 50,000 |  10,000 |        about 60,000 |   about 500,000 |
| epic    | 10,000 | 80,000 |  20,000 |       about 100,000 | about 1,000,000 |

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

Compiler benchmarks must measure:

- parse time;
- lowering time;
- parser syntax tree memory;
- source AST allocation volume;
- validation time;
- schema validation time;
- block reference resolution time;
- ID uniqueness check time;
- markup validation time;
- POT extraction time;
- compiled asset serialization time;
- compiled asset size.

Initial target budgets on a typical developer laptop:

- small project compile: under 100 ms;
- medium project compile: under 1 s;
- large project compile: under 5 s;
- no superlinear blowups for ID checks, block resolution, or schema validation.

These are targets, not hard promises. If targets are missed, the benchmark report must make the cost
visible.

### 19.4 Runtime Benchmarks

Runtime benchmarks must measure:

- `start_scene`;
- `next` for line events;
- `next` for prompt events;
- choice selection by ID;
- condition evaluation dispatch overhead;
- deferred effect collection;
- immediate effect emission;
- blocking effect pause and acknowledgement;
- locale lookup overhead;
- session serialization;
- session deserialization;
- full scene traversal with fixture context.

Initial target budgets:

- `next` without condition evaluation: allocation-free or near allocation-free after asset load;
- line/prompt advancement: comfortably under 50 us per event in release builds;
- choice selection by ID: effectively O(1) or O(log n), never linear over all project choices;
- session save/load: proportional to session state, not compiled asset size;
- runtime traversal must not clone full compiled assets.

### 19.5 LSP and Editor Benchmarks

LSP performance must be measured because authoring quality is a core product goal.

Benchmarks must cover:

- initial project indexing;
- open file parse;
- incremental edit parse;
- diagnostics refresh;
- completion latency;
- go-to definition latency;
- rename block latency;
- memory usage for indexed projects.

Initial target budgets:

- completion response under 50 ms for small/medium projects;
- diagnostics update under 100 ms for typical single-file edits;
- large project indexing should be incremental and cancellable;
- editor operations must avoid reparsing the entire project when a file-local edit is sufficient.

The standalone GUI workbench must be measured on every declared platform using the same
representative fixture and authoring operations. Its evidence must cover cold and warm startup,
project open and index, source edit to diagnostics, schema and localisation view refresh, preview
transition, graph navigation, and idle/active memory. These measurements inform the native strategy
bake-off and the release baseline; cross-platform timing claims must identify the host profile
rather than presenting one operating system as universal evidence.

### 19.6 Engine Adapter Benchmarks

Engine adapters must measure:

- asset loading and conversion overhead;
- event emission overhead;
- active session update overhead per frame or tick;
- condition handler dispatch overhead through the host engine;
- generated typed effect event/signal conversion overhead.

An adapter should add negligible frame cost when no dialogue session is active.

### 19.7 Memory Metrics

Benchmarks must report memory-sensitive metrics where practical:

- syntax tree size during parser-heavy flows;
- source AST allocation volume;
- compiled asset size;
- peak compiler memory;
- runtime session size;
- LSP project index size;
- number of allocations during hot runtime traversal;
- number of clones of large strings or metadata vectors.

The runtime should prefer shared immutable compiled data plus compact session state.

### 19.8 Regression Policy

CI should run a fast, non-comparative benchmark smoke suite on pull requests that affect core Rust,
benchmark fixtures, or shared build inputs, and on the weekly complete CI run. Documentation-only
changes do not require benchmark builds. A fuller benchmark suite belongs on release branches or
scheduled jobs. The pull-request smoke suite must use the existing `crates/recite-benchmarks`
Criterion targets with `RECITE_BENCH_SCALES=tiny` and explicit compiler/runtime/preview/LSP bench
target commands. It proves that the tiny compiler, runtime, preview, and LSP benchmarks build and
execute quickly; it does not compare timings or enforce regression thresholds.

The benchmark lane additionally compares release LSP binaries against the change base on the same
Ubuntu 24.04 runner. The current harness and generated large fixture drive both binaries. Three
alternating pairs measure six real edit kinds in full and negotiated sync modes, eight query/action
kinds, shared-destination invalidation, fresh-process indexing/opening and peak RSS. Each has two
warmups and 21 recorded samples with exact output fingerprints. Warm latency regressions must exceed
both 20% and 2 ms, occur in at least two pairs, and recur in a second three-pair round. The first
round covers every workload. Confirmation repeats only affected probe families, with each family's
request order, warmups and sample counts unchanged. Each round records its exact coverage; startup
and peak-memory observations remain coupled. Fresh-process readiness uses 30% plus 50 ms; document
opening uses 30% plus 10 ms; peak RSS through indexing/opening uses 20% plus 16 MiB. The
shared-destination fixture has 100 files, 2,000 blocks and 20,000 dialogue lines with references to
ten shared files. Fresh-process samples use a warm filesystem, not cold storage.

Incomplete or inconsistent evidence fails the check rather than reporting success. The checked-in
policy is `scripts/lsp-performance-policy.json`; raw samples and revision/binary identities are
retained as CI artifacts. Full and ranged sustained sessions check settlement and continued service.
A burst probe sends 25 edits at 5 ms intervals separated by 150 ms pauses; diagnostics and
successful completion, definition, rename and fix-all responses must recover within 500 ms of the
last edit. Freshness checks remain mandatory under overload.

This gate protects the measured Linux process workloads. It does not establish cross-platform editor
rendering or long-lived memory budgets. Issue #109 still owns the named release/scheduled benchmark
baseline and fuller regression suite; issue #77 owns the evidence ledger and release-gate decision
that consume its results. Those broader guarantees must not be inferred from the smoke or paired LSP
comparison.

Regression thresholds must be explicit and reviewable. They become blocking only when measured
against an agreed baseline and execution profile, such as a stable Linux runner or documented
release-measurement profile. Before those baselines exist, exceeding a threshold is a review trigger
rather than an automatic failure. The paired LSP gate above supplies an explicit comparative
baseline and rerun policy for its covered operations.

Initial regression review thresholds:

- more than 10% regression in hot runtime paths;
- more than 20% regression in compiler/LSP paths;
- any accidental O(n^2) behaviour on medium or large fixtures;
- unexpected allocation increases in allocation-sensitive runtime benchmarks.

Benchmark thresholds must be adjustable as the implementation matures, but changes to thresholds
should be reviewed explicitly. GUI performance and accessibility evidence must also record blocking
regressions (for example, keyboard/focus loss, screen-reader dead ends, unusable zoom, or an
interaction that becomes impossible under high contrast), even where no numeric threshold is
enforced.

### 19.9 Trace Metrics

`recite trace` should optionally emit performance counters:

- event count;
- line count;
- prompt count;
- choice count;
- condition evaluation count;
- effect count by mode;
- localization lookup count;
- elapsed traversal time;
- maximum serialized session size.

This makes real project dialogue scenes measurable without requiring users to write Rust benchmarks.
