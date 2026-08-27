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

The first structural ast-grep rule is intentionally narrow: it enforces the
existing private-test sidecar placement policy. File size, cohesion, generated
boundaries, and issue-backed exceptions are enforced by the companion
maintainability script. No broad style or nesting rules are enabled.

## Prior maintenance decisions

The evidence recorded while re-founding the v1 roadmap changes how these gates
should be used:

- #117's generic condition/effect model remains rejected as an implicit
  refactor target. The current schema keeps conditions as pure queries and
  effects as typed requests; the issue's accepted follow-up says any internal
  deduplication needs current duplication and readability evidence. This gate
  therefore records pressure and catches accidental boundary drift, but does
  not demand a generic abstraction.
- #136's original unused-LSP-snapshot premise is stale. `features.rs` consumes
  `LiveProjectSnapshot::summaries()`, `server.rs` consumes workspace generation,
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

## Inventory

| Path | Lines | Kind | Owner | Disposition | Issue/reason |
| --- | ---: | --- | --- | --- | --- |
| `crates/recite-ffi/src/lib.rs` | 839 | production | ffi | follow-up | #164/#171: separate ABI façade only after symbol and threading inventory |
| `crates/recite-core/src/schema/model/canonical.rs` | 621 | production | core/schema | review | Canonical model boundary; preserve public fingerprints |
| `crates/recite-core/src/schema/manifest/lower/availability.rs` | 560 | production | core/schema | review | Reassess mapping and domain-validation ownership |
| `crates/recite-core/src/compiled/messagepack/tags.rs` | 506 | production | core/wire | cohesive | #89: explicit wire tag table |
| `crates/recite-core/src/compiled/messagepack/wire.rs` | 505 | production | core/wire | exception | #89: retain the explicit decoder boundary while the v0 wire contract is synchronized |
| `crates/recite-runtime/src/session_snapshot.rs` | 293 | production | runtime/snapshot | follow-up | #135: typed snapshot-boundary errors |
| `crates/recite-core/src/compiled/messagepack/validate.rs` | 479 | production | core/wire | cohesive | #89: validation is part of the decoder boundary |
| `crates/recite-lsp/src/workspace.rs` | 479 | production | lsp/workspace | follow-up | #164: separate document state, saved indexes, and analysis snapshots |
| `crates/recite-compiler/src/wire/messagepack.rs` | 397 | production | compiler/wire | cohesive | #89: encoder mirror of the explicit decoder wire surface |
| `crates/recite-cli/src/error.rs` | 394 | production | cli | review | Keep user-facing error projection separate from typed domain errors |
| `crates/recite-lsp/src/features/navigation.rs` | 394 | production | lsp/features | review | Feature-specific navigation projection |
| `crates/recite-lsp/src/features/completion.rs` | 393 | production | lsp/features | review | Feature-specific completion and precedence handling |
| `crates/recite-benchmarks/src/report/mod.rs` | 389 | production | benchmarks | cohesive | Report aggregation boundary |
| `crates/recite-benchmarks/src/report/fixture.rs` | 387 | production | benchmarks | cohesive | Fixture report model |
| `crates/recite-cli/src/runtime_fixture/execute.rs` | 383 | production | cli/runtime-fixture | review | Keep headless execution separate from rendering |
| `crates/recite-godot/src/convert.rs` | 383 | production | godot | review | Host conversion boundary |
| `crates/recite-lsp/src/server.rs` | 383 | production | lsp/server | follow-up | #164: make request dispatch and workspace ownership explicit |
| `crates/recite-benchmarks/src/memory_profiles/mod.rs` | 373 | production | benchmarks | cohesive | Maintainer-only profile orchestration |
| `crates/recite-cli/src/commands.rs` | 372 | production | cli | review | Command orchestration boundary |
| `crates/recite-compiler/src/wire/messagepack/tags.rs` | 373 | production | compiler/wire | cohesive | #89: encoder tag mirror |
| `crates/recite-godot/src/adapter.rs` | 364 | production | godot | review | Host adapter lifecycle boundary |
| `crates/recite-core/src/schema/model/mod.rs` | 361 | production | core/schema | review | Schema module exports and model grouping |
| `crates/recite-cli/src/play/driver.rs` | 350 | production | cli/play | cohesive | Shared preview driver seam |
| `crates/recite-runtime/src/traversal/availability.rs` | 349 | production | runtime/traversal | cohesive | Deterministic availability traversal |
| `crates/recite-cli/src/play/tui/mod.rs` | 347 | production | cli/tui | review | TUI integration boundary |
| `crates/recite-core/src/schema/manifest/raw.rs` | 345 | production | core/schema | cohesive | Lossless raw manifest model |
| `crates/recite-compiler/src/compile/builder/rows.rs` | 337 | production | compiler | cohesive | Compiled row construction |
| `crates/recite-compiler/src/pot.rs` | 333 | production | compiler/localisation | follow-up | #164: consume shared authoring analysis |
| `crates/recite-benchmarks/src/id_metrics.rs` | 332 | production | benchmarks | cohesive | Maintainer metric calculations |
| `crates/recite-ffi/src/output.rs` | 328 | production | ffi | follow-up | #135: preserve typed failures to the C boundary |
| `crates/recite-runtime/src/traversal/asset.rs` | 328 | production | runtime/traversal | cohesive | Asset validation and traversal boundary |
| `crates/recite-cli/src/i18n/messages.rs` | 323 | production | cli/i18n | follow-up | #166: shared Fluent resource ownership |
| `crates/recite-compiler/src/wire/inspection.rs` | 312 | production | compiler/wire | review | Structured wire inspection projection |
| `crates/recite-benchmarks/src/project.rs` | 310 | production | benchmarks | cohesive | Synthetic project model |
| `crates/recite-cli/src/play/plain.rs` | 309 | production | cli/play | cohesive | Plain preview adapter |
| `crates/recite-cli/src/play/tui/state.rs` | 307 | production | cli/tui | cohesive | TUI reducer state |
| `crates/recite-compiler/src/compile/builder.rs` | 306 | production | compiler | cohesive | Compiled asset builder |
| `crates/recite-lsp/src/features/completion/projection.rs` | 296 | production | lsp/features | review | Completion projection |
| `crates/recite-core/src/schema/manifest/spans.rs` | 295 | production | core/schema | cohesive | Source-span calculation |
| `crates/recite-lsp/src/features.rs` | 295 | production | lsp/features | review | Intentional ordered feature lookup |
| `crates/recite-cli/src/play/tui/interaction.rs` | 293 | production | cli/tui | cohesive | Input-to-intent translation |
| `crates/recite-core/src/diagnostic.rs` | 289 | production | core/diagnostics | cohesive | Shared structured diagnostic surface |
| `crates/recite-compiler/src/validation/metadata.rs` | 285 | production | compiler/validation | review | Metadata validation ownership |
| `crates/recite-core/src/schema/manifest/lower/functions.rs` | 285 | production | core/schema | review | Function declaration lowering |
| `crates/recite-compiler/src/validation/conditions.rs` | 282 | production | compiler/validation | cohesive | Condition validation |
| `crates/recite-core/src/diagnostic/explanation/validation.rs` | 278 | production | core/diagnostics | cohesive | Diagnostic explanation catalog |
| `crates/recite-cli/src/play/tui/render/prompt.rs` | 270 | production | cli/tui | cohesive | Prompt rendering |
| `crates/recite-cli/src/tui/config.rs` | 268 | production | cli/tui | follow-up | #167: replace manual config locations with OS-aware loading |
| `crates/recite-cli/src/cli_help.rs` | 267 | production | cli | cohesive | CLI help presentation |
| `crates/recite-godot/src/bindings.rs` | 267 | production | godot | review | Host binding declarations |
| `crates/recite-cli/src/dialogue_locale/po.rs` | 266 | production | cli/localisation | follow-up | #166: source-preserving PO ownership |
| `crates/recite-cli/src/runtime_fixture/prompt.rs` | 266 | production | cli/runtime-fixture | cohesive | Fixture prompt projection |
| `crates/recite-benchmarks/src/runtime.rs` | 265 | production | benchmarks | cohesive | Runtime benchmark harness |
| `crates/recite-lsp/src/summary/file/collector.rs` | 256 | production | lsp/summary | review | File summary projection |

| `crates/recite-runtime/tests/adapter_conformance/driver.rs` | 1153 | test/support | runtime/tests | cohesive | Shared adapter conformance driver |
| `crates/recite-cli/src/play/tui/render/tests.rs` | 811 | test/support | cli/tui/tests | cohesive | Private rendering contract tests |
| `crates/recite-cli/tests/runtime.rs` | 759 | test/support | cli/tests | cohesive | Runtime command behavior suite |
| `crates/recite-core/tests/compiled_messagepack.rs` | 693 | test/support | core/tests | cohesive | Wire compatibility contract |
| `crates/recite-core/tests/schema_manifest/fingerprint.rs` | 638 | test/support | core/tests | cohesive | Canonical fingerprint contract |
| `crates/recite-core/tests/support/mod.rs` | 605 | test/support | core/tests | cohesive | Shared model and wire constructors |
| `crates/recite-runtime/tests/adapter_conformance/manifest.rs` | 571 | test/support | runtime/tests | cohesive | Adapter manifest contract |
| `crates/recite-compiler/tests/asset.rs` | 564 | test/support | compiler/tests | exception | #89: retain the shared compiled-asset fixture entry point for the wire contract guard |
| `crates/recite-lsp/src/tests/support.rs` | 423 | test/support | lsp/tests | review | Test support ownership; retain private access where required |
| `crates/recite-lsp/src/tests/project_indexes.rs` | 422 | test/support | lsp/tests | review | Private index behavior |
| `crates/recite-core/tests/compiled_model.rs` | 419 | test/support | core/tests | cohesive | Compiled model behavior |
| `crates/recite-compiler/tests/asset/tag_surface.rs` | 486 | test/support | compiler/tests | cohesive | Wire tag surface |
| `crates/recite-runtime/tests/traversal/localisation.rs` | 411 | test/support | runtime/tests | cohesive | Locale traversal contract |
| `crates/recite-parser/tests/parser/lowering.rs` | 406 | test/support | parser/tests | cohesive | Lowering behavior suite |
| `crates/recite-compiler/tests/pot_extraction.rs` | 404 | test/support | compiler/tests | follow-up | #166: shared localisation extraction fixtures |
| `crates/recite-lsp/src/tests/diagnostics.rs` | 402 | test/support | lsp/tests | review | Private diagnostic projection tests |
| `crates/recite-fixturegen/tests/generation.rs` | 395 | test/support | fixturegen/tests | cohesive | Deterministic generation contract |
| `crates/recite-runtime/tests/adapter_conformance.rs` | 383 | test/support | runtime/tests | cohesive | Adapter conformance entry point |
| `crates/recite-core/tests/schema_manifest/load_valid.rs` | 379 | test/support | core/tests | cohesive | Valid manifest coverage |
| `crates/recite-runtime/tests/session_serialization/invalid_snapshots.rs` | 378 | test/support | runtime/tests | cohesive | Snapshot failure contract |
| `crates/recite-parser/tests/parser/statements.rs` | 374 | test/support | parser/tests | cohesive | Statement parser coverage |
| `crates/recite-cli/tests/watch_stress.rs` | 366 | test/support | cli/tests | cohesive | Watch stress harness |
| `crates/recite-runtime/tests/traversal/conditions/choice_conditions.rs` | 365 | test/support | runtime/tests | cohesive | Choice condition coverage |
| `crates/recite-lsp/src/tests.rs` | 369 | test/support | lsp/tests | cohesive | Module aggregator, not production implementation |
| `crates/recite-lsp/src/tests/code_action/schema_entry.rs` | 357 | test/support | lsp/tests | review | Private code-action behavior |
