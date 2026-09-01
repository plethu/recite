# Maintainability baseline

This is the checked-in inventory for Recite's handwritten Rust surfaces. It is
an ownership and review aid, not a demand that every large file be split.

The maintainability check compares the source file at the pull request head
with the merge base. Existing debt is reported but does not fail a change when
it is unchanged or smaller. A new file above the scrutiny threshold needs an
entry here. A changed file that crosses or grows beyond the follow-up
threshold needs a real split or a narrowly scoped `exception` entry with an
issue and reason. An exception never authorises growth of an unrelated file.

## Thresholds and classification

| Surface | Scrutinise above | Follow up above |
| --- | ---: | ---: |
| Production Rust under `crates/*/src/` | 250 lines | 400 lines |
| Test/support Rust under crate tests, `crates/*/benches/`, `src/**/tests.rs`, or `tests/` | 350 lines | 500 lines |

`tests.rs` sidecars, `src/tests/`, crate integration tests, and crate benches
are test/support code even though some are below a crate's `src/` directory.
Generated headers, build output, and generated fixture output are outside this
inventory. Any future generated Rust path must be excluded by an explicit,
reviewed path rule; a broad generated directory exemption is not acceptable.

The `kind` column must agree with the path classification (`production` or
`test/support`), and the recorded line count must match the checked-out head.
The `disposition` column is deliberately descriptive. `cohesive` means the
current boundary is understood and should not be split for a number alone;
`follow-up` names work that should reduce change pressure; `review` means a
future change should reassess ownership before adding responsibility. None of
these dispositions permit a file to grow past its follow-up threshold. The
only growth exemption is a row explicitly marked `exception`, with a positive
issue reference such as `#164` and a concrete reason. Baseline validation is
syntax-only and never queries GitHub. The inventory is intentionally one
pipe-delimited Markdown table: keep six cells, use backticks around paths, do
not put `|` in reasons, and keep issue references in `#N` syntax. The checker
rejects malformed rows rather than attempting Markdown escaping.

When an intentional shrink takes a file to or below its scrutiny threshold,
remove its row in the same change. If it remains above scrutiny, update the
recorded line count and reassess its disposition. Every validation run rejects
stale rows, missing paths, missing oversized-file rows, and line-count drift so
the inventory cannot silently become historical fiction.

The changed-surface check compares a pull request's source head with its
merge base. For a push to `main`, CI passes `github.event.before` through the
`RECITE_BASE_REF` environment variable. An all-zero initial-push SHA is
treated as Git's empty tree, so the first push is checked without relying on
`HEAD^`. Rename and copy changes compare the destination against the original
base path, so moving a large file without changing it is not mistaken for new
growth. Local runs default to `origin/main` unless explicit refs are passed.

The structural ast-grep rules are intentionally narrow. `rust-inline-test-module`
enforces the existing private-test sidecar placement policy. The
`rust-elseif-cascade` rule catches four or more equality branches over the same
value, where policy ownership can become difficult to see; three-way
classifiers stay allowed because they are common and usually remain local.
Neither rule uses file size as a proxy for cohesion, and no broad style or
nesting rules are enabled. File size, cohesion, generated boundaries, and
issue-backed exceptions are enforced by the companion maintainability script.

An intentionally ordered four-equality-branch chain can use a local
`// ast-grep-ignore: rust-elseif-cascade` comment. Keep the comment adjacent to
the chain and explain why its order is the local policy; do not disable the
rule for a directory or repository-wide.

## Prior maintenance decisions

The evidence recorded while re-founding the v1 roadmap changes how these gates
should be used:

- #117's generic condition/effect model remains rejected as an implicit
  refactor target. The current schema keeps conditions as pure queries and
  effects as typed requests; the issue's accepted follow-up says any internal
  deduplication needs current duplication and readability evidence. This gate
  therefore records pressure and catches accidental boundary drift, but does
  not demand a generic abstraction.
- #136's original unused-LSP-snapshot premise is stale. `workspace.rs` builds
  code-action documents from `LiveProjectSnapshot::summaries()`, `server.rs`
  consumes workspace generation,
  and the benchmark and test support paths exercise both accessors. No change
  is warranted for that issue; any remaining `allow(dead_code)` is a separate,
  evidence-backed review item rather than a reason to delete the live snapshot.

The gates cover maintainability evidence across the public schema and runtime
surfaces, the FFI boundary, generated-artifact exclusions, test placement and
module boundaries, and deterministic wire/serialization files. They do so by
classifying paths, requiring explicit baseline ownership, checking the focused
private-test placement pattern, and retaining existing project/API/FFI,
serialization, and test gates. They do not prove semantics, API compatibility,
FFI safety, or deterministic behaviour from line counts or ast-grep alone;
those claims remain the responsibility of the typed tests, compile gates,
wire compatibility fixtures, and focused reviews for each subsystem.

Lint suppressions have a separate diff-aware inventory and policy in
[`docs/lint-suppression-policy.md`](lint-suppression-policy.md). Keep that
policy's baseline/current distinction separate from this line-count inventory:
unchanged suppression debt is reported, while new or expanded handwritten
production suppressions must carry a narrow scope and rationale.

## Inventory

| Path | Lines | Kind | Owner | Disposition | Issue/reason |
| --- | ---: | --- | --- | --- | --- |
| `crates/recite-core/src/schema/model/canonical.rs` | 619 | production | core/schema | review | Canonical model boundary; preserve public fingerprints |
| `crates/recite-core/src/schema/manifest/lower/availability_bindings.rs` | 304 | production | core/schema | review | #182/#183: typed availability reason literal and condition-parameter bindings |
| `crates/recite-core/src/schema/manifest/lower/availability/mapping.rs` | 298 | production | core/schema | review | #182: availability reason mapping validation and source-aware lowering |
| `crates/recite-core/src/schema/manifest/lower/domains_provenance.rs` | 285 | production | core/schema | review | Domain provenance lowering owns flat and contextual provenance shapes |
| `crates/recite-core/src/schema/manifest/lower/producer_provenance.rs` | 303 | production | core/schema | review | Producer origin and fingerprint lowering owns source-aware provenance paths |
| `crates/recite-core/src/compiled/messagepack/tags.rs` | 468 | production | core/wire | cohesive | #89: explicit wire tag table |
| `crates/recite-core/src/compiled/messagepack/wire.rs` | 662 | production | core/wire | exception | #89: retain the explicit decoder boundary while the v0 wire contract is synchronized |
| `crates/recite-runtime/src/session_snapshot.rs` | 294 | production | runtime/snapshot | follow-up | #135: typed snapshot-boundary errors |
| `crates/recite-cli/src/error.rs` | 372 | production | cli | review | Keep user-facing error projection separate from typed domain errors |
| `crates/recite-ui/tests/diagnostics.rs` | 371 | test/support | ui/tests | review | Structured diagnostic resource and compatibility-message coverage |
| `crates/recite-compiler/tests/authoring_edits.rs` | 387 | test/support | compiler/tests | review | Host-neutral authoring edit application and scoped/range planning coverage |
| `crates/recite-lsp/src/tests/code_action/missing_id.rs` | 363 | test/support | lsp/tests | review | Stable-ID code-action protocol coverage, including guarded project preconditions |
| `crates/recite-lsp/src/tests/availability/completion.rs` | 373 | test/support | lsp/tests | review | #176: contextual selector completion coverage |
| `crates/recite-benchmarks/src/report/mod.rs` | 389 | production | benchmarks | cohesive | Report aggregation boundary |
| `crates/recite-benchmarks/src/report/fixture.rs` | 387 | production | benchmarks | cohesive | Fixture report model |
| `crates/recite-cli/src/dialogue_locale/catalog.rs` | 284 | production | cli/localisation | review | #180/#191: catalogue loading and validated plural-arm evidence remain a cohesive provider boundary |
| `crates/recite-cli/src/runtime_fixture/trace/model.rs` | 263 | production | cli/runtime-fixture | review | #180: trace output model keeps localized templates and structured metadata distinct |
| `crates/recite-godot/src/adapter.rs` | 361 | production | godot | review | Host adapter lifecycle boundary |
| `crates/recite-lsp/src/server.rs` | 398 | production | lsp/server | review | #164: request dispatch and protocol lifecycle remain the server boundary; error taxonomy and watcher registration are split into focused modules |
| `crates/recite-lsp/src/workspace/lsp_features.rs` | 253 | production | lsp/workspace | review | #187: partition-routed authoring feature projections remain a cohesive LSP boundary; split by feature family if responsibilities grow materially |
| `crates/recite-benchmarks/src/memory_profiles/mod.rs` | 373 | production | benchmarks | cohesive | Maintainer-only profile orchestration |
| `crates/recite-cli/src/commands.rs` | 398 | production | cli | review | Command orchestration boundary |
| `crates/recite-compiler/src/authoring/query/schema/metadata.rs` | 269 | production | compiler/authoring | review | #168: typed metadata completion and missing-context policy projection |
| `crates/recite-core/src/schema/model/mod.rs` | 382 | production | core/schema | review | Schema module exports and model grouping |
| `crates/recite-core/src/schema/source/export/basic.rs` | 284 | production | core/schema | cohesive | Deterministic JSON export for basic schema declarations |
| `crates/recite-core/src/schema/source/lower/mod.rs` | 306 | production | core/schema | review | TOML source normalization and shared canonical lowering entrypoint |
| `crates/recite-runtime/src/traversal/availability.rs` | 370 | production | runtime/traversal | cohesive | Deterministic availability traversal |
| `crates/recite-runtime/src/traversal/interpolation.rs` | 443 | production | runtime/traversal | exception | #179/#180/#191: typed interpolation and plural localisation retain the provider arm-boundary handoff |
| `crates/recite-runtime/src/preview/driver.rs` | 253 | production | runtime/preview | review | #191: preview event projection carries validated plural-arm evidence into snapshot state |
| `crates/recite-runtime/src/preview/snapshot_validation.rs` | 262 | production | runtime/preview | review | #191: snapshot prompt validation keeps plural provenance and arm bounds mutually consistent |
| `crates/recite-runtime/tests/preview_snapshot.rs` | 481 | test/support | runtime/preview | review | #191: hostile preview snapshot and plural-arm wire coverage |
| `crates/recite-core/src/schema/manifest/raw.rs` | 391 | production | core/schema | cohesive | Lossless raw manifest model |
| `crates/recite-compiler/src/compile/builder/rows.rs` | 353 | production | compiler | cohesive | Compiled row construction |
| `crates/recite-compiler/src/pot.rs` | 366 | production | compiler/localisation | follow-up | #164: consume shared authoring analysis |
| `crates/recite-benchmarks/src/id_metrics.rs` | 332 | production | benchmarks | cohesive | Maintainer metric calculations |
| `crates/recite-runtime/src/traversal/asset.rs` | 328 | production | runtime/traversal | cohesive | Asset validation and traversal boundary |
| `crates/recite-runtime/src/traversal/output.rs` | 279 | production | runtime/traversal | review | #180: structured plural output construction remains beside traversal until the output boundary settles |
| `crates/recite-compiler/src/wire/inspection.rs` | 337 | production | compiler/wire | review | Structured wire inspection projection |
| `crates/recite-benchmarks/src/project.rs` | 310 | production | benchmarks | cohesive | Synthetic project model |
| `crates/recite-cli/src/play/tui/state.rs` | 307 | production | cli/tui | cohesive | TUI reducer state |
| `crates/recite-compiler/src/compile/builder.rs` | 306 | production | compiler | cohesive | Compiled asset builder |
| `crates/recite-core/src/schema/manifest/spans.rs` | 328 | production | core/schema | cohesive | JSON span calculation and shared span state |
| `crates/recite-cli/src/play/tui/interaction.rs` | 293 | production | cli/tui | cohesive | Input-to-intent translation |
| `crates/recite-compiler/src/validation/metadata.rs` | 276 | production | compiler/validation | review | Metadata validation ownership |
| `crates/recite-compiler/src/validation/statements.rs` | 329 | production | compiler/validation | cohesive | Statement traversal owns per-class validation gates; interpolation and plural validation remain separate seams |
| `crates/recite-core/src/schema/manifest/lower/domains.rs` | 342 | production | core/schema | review | Strict domain shape and declaration lowering |
| `crates/recite-core/src/schema/manifest/validate.rs` | 278 | production | core/schema | review | Shared schema reference and name validation boundary |
| `crates/recite-compiler/src/validation/conditions.rs` | 282 | production | compiler/validation | cohesive | Condition validation |
| `crates/recite-core/src/diagnostic/explanation/validation.rs` | 352 | production | core/diagnostics | cohesive | Diagnostic explanation catalog |
| `crates/recite-cli/src/play/tui/render/prompt.rs` | 270 | production | cli/tui | cohesive | Prompt rendering |
| `crates/recite-cli/src/cli_help.rs` | 330 | production | cli | cohesive | CLI help presentation |
| `crates/recite-godot/src/bindings.rs` | 353 | production | godot | review | Host binding declarations |
| `crates/recite-benchmarks/src/runtime.rs` | 265 | production | benchmarks | cohesive | Runtime benchmark harness |

| `crates/recite-runtime/tests/adapter_conformance/driver.rs` | 1179 | test/support | runtime/tests | exception | #171: shared conformance driver grows with typed callback scenarios; retain until adapter conformance split |
| `crates/recite-cli/src/play/tui/render/tests.rs` | 811 | test/support | cli/tui/tests | cohesive | Private rendering contract tests |
| `crates/recite-cli/tests/runtime.rs` | 659 | test/support | cli/tests | cohesive | Runtime command behavior suite |
| `crates/recite-cli/tests/dialogue_locale.rs` | 480 | test/support | cli/tests | review | #180: end-to-end locale, plural, and trace scenarios remain grouped by the CLI contract |
| `crates/recite-core/tests/support/mod.rs` | 554 | test/support | core/tests | cohesive | Shared model and wire constructors |
| `crates/recite-core/tests/schema_manifest/fingerprint.rs` | 641 | test/support | core/tests | exception | #176: retain the canonical fingerprint fixture while typed provenance constructors migrate; producer-specific assertions are split into a dedicated test |
| `crates/recite-ffi/tests/conditions.rs` | 437 | test/support | ffi/tests | cohesive | #171: condition callback protocol coverage |
| `crates/recite-ffi/tests/interpolation.rs` | 402 | test/support | ffi/tests | cohesive | #179: typed interpolation adapter traversal coverage |
| `crates/recite-ffi/tests/localisation.rs` | 368 | test/support | ffi/tests | cohesive | #166: shared locale callback fixtures and translated/fallback traversal coverage |
| `crates/recite-ffi/src/session/start.rs` | 343 | production | ffi/session | cohesive | #166: provider-backed start ownership and rollback remain one cohesive session boundary |
| `crates/recite-ffi/src/locale/provider.rs` | 273 | production | ffi/locale | cohesive | #166: owned callback provider request and result parsing remain one cohesive FFI boundary |
| `crates/recite-ffi/src/session/restore.rs` | 284 | production | ffi/session | review | #166: provider-backed restore ownership and rollback boundary |
| `crates/recite-godot/src/catalog.rs` | 313 | production | godot | review | #166: owned locale catalogue and deterministic provider resolution |
| `crates/recite-godot/src/catalog_resource.rs` | 389 | production | godot | review | #166: serializable Resource catalogue boundary and validated rebuild |
| `crates/recite-runtime/tests/interpolation.rs` | 498 | test/support | runtime/tests | review | #180: typed interpolation and plural provider scenarios remain grouped around runtime delivery |
| `crates/recite-compiler/tests/asset.rs` | 640 | test/support | compiler/tests | exception | #89: retain the shared compiled-asset fixture entry point for the wire contract guard |
| `crates/recite-ffi/tests/snapshots.rs` | 422 | test/support | ffi/tests | cohesive | #171: session snapshot and restore contract coverage |
| `crates/recite-lsp/src/tests/support/harness.rs` | 434 | test/support | lsp/tests | follow-up | Protocol harness ownership remains grouped for shared request, response, and lifecycle helpers; split again if protocol coverage grows materially |
| `crates/recite-core/tests/compiled_model.rs` | 427 | test/support | core/tests | cohesive | Compiled model behavior |
| `crates/recite-compiler/tests/asset/tag_surface.rs` | 486 | test/support | compiler/tests | cohesive | Wire tag surface |
| `crates/recite-runtime/tests/traversal/localisation.rs` | 441 | test/support | runtime/tests | cohesive | Locale traversal contract |
| `crates/recite-parser/tests/parser/lowering.rs` | 406 | test/support | parser/tests | cohesive | Lowering behavior suite |
| `crates/recite-compiler/tests/pot_extraction.rs` | 461 | test/support | compiler/tests | follow-up | #166: shared localisation extraction fixtures |
| `crates/recite-fixturegen/tests/generation.rs` | 416 | test/support | fixturegen/tests | cohesive | Deterministic generation contract |
| `crates/recite-ffi/tests/lifecycle.rs` | 399 | test/support | ffi/tests | cohesive | #171: session lifecycle and begin retry coverage |
| `crates/recite-runtime/tests/adapter_conformance.rs` | 426 | test/support | runtime/tests | cohesive | Adapter conformance entry point |
| `crates/recite-core/tests/schema_manifest/load_valid.rs` | 452 | test/support | core/tests | cohesive | Valid manifest coverage |
| `crates/recite-runtime/tests/session_serialization/invalid_snapshots.rs` | 478 | test/support | runtime/tests | cohesive | Snapshot failure contract |
| `crates/recite-cli/tests/watch_stress.rs` | 366 | test/support | cli/tests | cohesive | Watch stress harness |
| `crates/recite-cli/src/watch/build/tests.rs` | 359 | test/support | cli/watch/build-tests | review | #189: status and telemetry coverage is split; keep each responsibility below the follow-up threshold |
| `crates/recite-cli/src/watch/tests.rs` | 377 | test/support | cli/watch-tests | review | #189: event and initial-build integration coverage remains one watch contract; reassess before adding another responsibility |
| `crates/recite-compiler/tests/authoring_build/status_projection.rs` | 419 | test/support | compiler/authoring-build-tests | review | #189: lifecycle projection and non-semantic telemetry assertions remain one focused contract suite |
| `crates/recite-runtime/tests/traversal/conditions/choice_conditions.rs` | 365 | test/support | runtime/tests | cohesive | Choice condition coverage |
| `crates/recite-lsp/src/tests/availability/speaker.rs` | 351 | test/support | lsp/tests | review | Typed and ordinary speaker completion coverage |
| `crates/recite-compiler/tests/validation/participation.rs` | 369 | test/support | compiler/tests | cohesive | #168: participation-aware validation completeness and all-complete compatibility coverage |
