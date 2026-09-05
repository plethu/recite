# Maintainability baseline

This is the checked-in inventory for Recite's handwritten source surfaces. It
is an ownership and review aid, not a demand that every large file be split.

The maintainability check compares the source file at the pull request head
with the merge base. Existing debt is reported but does not fail a change when
it is unchanged or smaller. A new file above the scrutiny threshold needs an
entry here. A changed file that crosses or grows beyond the follow-up
threshold needs a real split or a narrowly scoped `exception` entry with an
issue and reason. An exception never authorises growth of an unrelated file.

## Thresholds and classification

| Surface | Scrutinise above | Follow up above |
| --- | ---: | ---: |
| Production source, including editor runtime/client/grammar code | 250 lines | 400 lines |
| Tooling and gate source under `scripts/`, editor `scripts/`, or agent `scripts/` | 250 lines | 400 lines |
| Test/support source under crate tests, `crates/*/benches/`, `src/**/tests.rs`, `tests/`, or editor test dirs | 350 lines | 500 lines |

`tests.rs` sidecars, `src/tests/`, crate integration tests, crate benches, and
editor test directories are test/support code even though some are below a
source directory. `scripts/`, editor build/check scripts, and agent-local
scripts are tooling. The supported handwritten extensions are `.rs`, `.js`,
`.mjs`, `.cjs`, `.lua`, `.py`, and `.sh`.

Generated headers, build output, and generated fixture output are outside this
inventory. The checker names generated paths explicitly: the FFI header,
Tree-sitter parser/grammar/node-type outputs, the VS Code message projection,
and the Neovim message projection. VS Code's `dist/` directory is not an
exemption: any force-tracked source there is governed as production code. Any
future generated path must be added as an explicit reviewed rule; a broad
generated-directory exemption is not acceptable.

The `kind` column must agree with the path classification (`production`,
`tooling`, or `test/support`), and the recorded line count must match the
checked-out head.
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
| `crates/recite-cli/src/error.rs` | 386 | production | cli | review | Keep user-facing error projection separate from typed domain errors |
| `crates/recite-cli/src/error/user_message.rs` | 266 | production | cli | review | Localized CLI error presentation remains a dedicated user-message boundary |
| `crates/recite-cli/src/structured/error_mapping.rs` | 382 | production | cli/structured | cohesive | Exhaustive CliError-to-wire classification remains one stable protocol boundary |
| `crates/recite-ui/tests/diagnostics.rs` | 371 | test/support | ui/tests | review | Structured diagnostic resource and compatibility-message coverage |
| `crates/recite-compiler/tests/authoring_edits.rs` | 387 | test/support | compiler/tests | review | Host-neutral authoring edit application and scoped/range planning coverage |
| `crates/recite-lsp/src/tests/code_action/missing_id.rs` | 363 | test/support | lsp/tests | review | Stable-ID code-action protocol coverage, including guarded project preconditions |
| `crates/recite-lsp/tests/stdio_retired_schema_aliases.rs` | 380 | test/support | lsp/tests | review | #187: exact stdio retirement and late-alias lifecycle coverage |
| `crates/recite-lsp/src/tests/availability/completion.rs` | 373 | test/support | lsp/tests | review | #176: contextual selector completion coverage |
| `crates/recite-benchmarks/src/report/mod.rs` | 389 | production | benchmarks | cohesive | Report aggregation boundary |
| `crates/recite-benchmarks/src/report/fixture.rs` | 387 | production | benchmarks | cohesive | Fixture report model |
| `crates/recite-cli/src/dialogue_locale/catalog.rs` | 284 | production | cli/localisation | review | #180/#191: catalogue loading and validated plural-arm evidence remain a cohesive provider boundary |
| `crates/recite-cli/src/runtime_fixture/trace/model.rs` | 263 | production | cli/runtime-fixture | review | #180: trace output model keeps localized templates and structured metadata distinct |
| `crates/recite-godot/src/adapter.rs` | 361 | production | godot | review | Host adapter lifecycle boundary |
| `crates/recite-lsp/src/server.rs` | 279 | production | lsp/server | review | #164: request dispatch and protocol lifecycle remain the server boundary; notification lifecycle handlers are split into a focused protocol module |
| `crates/recite-lsp/src/workspace.rs` | 254 | production | lsp/workspace | review | #187: workspace lifecycle state and diagnostic refresh contracts remain the cohesive workspace boundary |
| `crates/recite-lsp/src/workspace/kernel_rebuild.rs` | 300 | production | lsp/workspace | review | #187: partition rebuild and fingerprint ownership remain a cohesive transactional workspace boundary |
| `crates/recite-lsp/src/workspace/project_index.rs` | 343 | production | lsp/workspace | review | #187: discovery documents, diagnostics, and per-partition completeness remain the cohesive saved-index boundary |
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
| `crates/recite-compiler/src/validation/statements.rs` | 341 | production | compiler/validation | cohesive | Statement traversal owns per-class validation gates; interpolation and plural validation remain separate seams |
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
| `crates/recite-cli/tests/dialogue_locale.rs` | 500 | test/support | cli/tests | review | #180: end-to-end locale, plural, trace, and structured trace locale projection scenarios remain grouped by the CLI contract |
| `crates/recite-cli/tests/structured_command.rs` | 490 | test/support | cli/tests | review | #53: finite structured command protocol conformance remains one external CLI suite; split before adding watch coverage |
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
| `crates/recite-cli/src/watch/build/tests.rs` | 367 | test/support | cli/watch/build-tests | review | #189: status and telemetry coverage is split; keep each responsibility below the follow-up threshold |
| `crates/recite-cli/src/watch/protocol.rs` | 302 | production | cli/watch/protocol | cohesive | Versioned watch stream orchestration owns lifecycle ordering and host cancellation coordination |
| `crates/recite-cli/src/watch/protocol/tests.rs` | 410 | test/support | cli/watch/protocol-tests | review | Active cancellation and stream-failure ordering use deterministic injected seams |
| `crates/recite-cli/src/watch/tests.rs` | 415 | test/support | cli/watch-tests | review | #189: event and initial-build integration coverage remains one watch contract; reassess before adding another responsibility |
| `crates/recite-cli/src/watch/build.rs` | 309 | production | cli/watch/build | review | #191: build orchestration retains the coordinator, publication, and post-publish freshness boundary; split if another lifecycle responsibility is added |
| `crates/recite-cli/src/watch/mod.rs` | 252 | production | cli/watch | review | Watch command routing keeps human and structured host entrypoints together; split if dispatch grows materially |
| `crates/recite-cli/src/watch/wire_mapping.rs` | 331 | production | cli/watch/wire | cohesive | Explicit compiler-state to version-1 DTO mapping owns publication, recovery, and failure projections |
| `crates/recite-cli/tests/structured_watch.rs` | 429 | test/support | cli/tests | review | #53: real-process structured watch lifecycle and cancellation transport coverage |
| `crates/recite-compiler/tests/authoring_build/status_projection.rs` | 492 | test/support | compiler/authoring-build-tests | review | #189: lifecycle projection and non-semantic telemetry assertions remain one focused contract suite |
| `crates/recite-compiler/src/authoring/build/result.rs` | 260 | production | compiler/authoring-build | review | #191: result finalization retains publication and freshness truth together; split if further lifecycle evidence is added |
| `crates/recite-runtime/tests/traversal/conditions/choice_conditions.rs` | 365 | test/support | runtime/tests | cohesive | Choice condition coverage |
| `crates/recite-lsp/src/tests/availability/speaker.rs` | 351 | test/support | lsp/tests | review | Typed and ordinary speaker completion coverage |
| `crates/recite-compiler/tests/validation/participation.rs` | 424 | test/support | compiler/tests | cohesive | #168: participation-aware validation completeness and all-complete compatibility coverage |
| `crates/recite-ui/tests/contract.rs` | 475 | test/support | ui/tests | review | #51: typed client projection and argument parity coverage remains one inventory contract suite |
| `editors/recite-tree-sitter/grammar.js` | 390 | production | tree-sitter/grammar | cohesive | Grammar source owns syntax and recovery rules alongside the named node declarations |
| `editors/vscode/src/controller.js` | 377 | production | vscode/controller | review | #51: controller retains restart coordination, startup projection, and terminal child-failure recovery |
| `editors/vscode/src/lsp-features.js` | 262 | production | vscode/lsp-features | review | #51: LSP range/workspace-edit conversion and version precondition projection remain one checked editor boundary |
| `editors/vscode/src/lsp-client.js` | 375 | production | vscode/lsp-client | review | #51: client keeps request settlement, child event ordering, transport closure, and bounded teardown as one shared lifecycle |
| `editors/vscode/test/lsp.test.mjs` | 460 | test/support | vscode/tests | review | #51: fake child, clock, framing, and lifecycle contract scenarios remain one protocol-boundary suite |
| `editors/vscode/test/controller-lifecycle.test.mjs` | 383 | test/support | vscode/tests | review | #51: controller startup, restart, capability, and explicit rename lifecycle coverage remain one host lifecycle suite; reassess before adding another lifecycle responsibility |
| `editors/vscode/scripts/message-projections.mjs` | 303 | tooling | vscode/projections | review | #51: inventory parsing, typed placeholder lowering, and projection installation remain one checked update boundary |
| `editors/vscode/scripts/ui-boundary-adapter.mjs` | 251 | tooling | vscode/checks | review | #51: semantic UI adapter contract remains one cohesive structural boundary |
| `editors/vscode/scripts/ui-boundary-command-contracts.mjs` | 257 | tooling | vscode/checks | review | #51: typed command and rename UI capabilities remain one explicit structural boundary; reassess before adding another host capability |
| `editors/vscode/scripts/ui-boundary-calls.mjs` | 350 | tooling | vscode/checks | review | UI boundary call inventory remains a single generated-boundary checker |
| `scripts/check-lint-suppressions.py` | 256 | tooling | lint-policy | review | Suppression policy parsing and diff-aware enforcement remain one checker boundary |
| `scripts/check-tree-sitter.sh` | 399 | tooling | tree-sitter/check | review | Parser generation, ABI, corpus, and reproducibility checks share one tool boundary |
| `scripts/check-zed.sh` | 304 | tooling | zed/check | review | #192: Zed manifest, grammar pin, task argv, launcher API, and parity evidence remain one checked boundary; split before adding another host surface |
| `scripts/lint_suppression_ast.py` | 374 | tooling | lint-policy | review | AST suppression extraction keeps parser traversal and source categorisation together |
| `tests/editor-parity/check.sh` | 495 | test/support | editor-parity/tests | review | Editor parity fixture scenarios remain one executable contract suite |
| `tests/lint-suppressions/check.sh` | 398 | test/support | lint-policy/tests | review | Hostile suppression-policy fixture scenarios remain one executable contract suite |
| `tests/maintainability/check.sh` | 376 | test/support | maintainability/tests | review | Core Rust threshold, baseline, zero-SHA, and inherited-debt fixtures remain one contract suite |
