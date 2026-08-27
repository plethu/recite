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
| Test/support Rust under crate tests, `src/**/tests.rs`, or `tests/` | 350 lines | 500 lines |

`tests.rs` sidecars and `src/tests/` are test/support code even though they
are below a crate's `src/` directory. Generated headers, build output, and
generated fixture output are outside this inventory. Any future generated Rust
path must be excluded by an explicit, reviewed path rule; a broad generated
directory exemption is not acceptable.

The `disposition` column is deliberately descriptive. `cohesive` means the
current boundary is understood and should not be split for a number alone;
`follow-up` names work that should reduce change pressure; `review` means a
future change should reassess ownership before adding responsibility. None of
these dispositions permit a file to grow past its follow-up threshold. The
only growth exemption is a row explicitly marked `exception`, with a linked
issue and a concrete reason.

## Production surfaces

| Path | Lines | Owner | Disposition | Issue/reason |
| --- | ---: | --- | --- | --- |
| `crates/recite-core/src/schema/manifest/lower/projection.rs` | 1,551 | core/schema | follow-up | #164: split projection lowering by authored responsibility |
| `crates/recite-ffi/src/lib.rs` | 838 | ffi | follow-up | #164/#171: separate ABI façade only after symbol and threading inventory |
| `crates/recite-core/src/schema/model/canonical.rs` | 621 | core/schema | review | Canonical model boundary; preserve public fingerprints |
| `crates/recite-core/src/schema/manifest/lower/availability.rs` | 560 | core/schema | review | Reassess mapping and domain-validation ownership |
| `crates/recite-core/src/compiled/messagepack/tags.rs` | 506 | core/wire | cohesive | #89: explicit wire tag table |
| `crates/recite-core/src/compiled/messagepack/wire.rs` | 502 | core/wire | cohesive | #89: decoder wire boundary |
| `crates/recite-runtime/src/session_snapshot.rs` | 482 | runtime/snapshot | follow-up | #135: typed snapshot-boundary errors |
| `crates/recite-core/src/compiled/messagepack/validate.rs` | 479 | core/wire | cohesive | #89: validation is part of the decoder boundary |
| `crates/recite-lsp/src/workspace.rs` | 479 | lsp/workspace | follow-up | #164: separate document state, saved indexes, and analysis snapshots |
| `crates/recite-compiler/src/wire/messagepack.rs` | 397 | compiler/wire | cohesive | #89: encoder mirror of the explicit decoder wire surface |
| `crates/recite-cli/src/error.rs` | 394 | cli | review | Keep user-facing error projection separate from typed domain errors |
| `crates/recite-lsp/src/features/navigation.rs` | 394 | lsp/features | review | Feature-specific navigation projection |
| `crates/recite-lsp/src/features/completion.rs` | 393 | lsp/features | review | Feature-specific completion and precedence handling |
| `crates/recite-benchmarks/src/report/mod.rs` | 389 | benchmarks | cohesive | Report aggregation boundary |
| `crates/recite-benchmarks/src/report/fixture.rs` | 387 | benchmarks | cohesive | Fixture report model |
| `crates/recite-cli/src/runtime_fixture/execute.rs` | 383 | cli/runtime-fixture | review | Keep headless execution separate from rendering |
| `crates/recite-godot/src/convert.rs` | 383 | godot | review | Host conversion boundary |
| `crates/recite-lsp/src/server.rs` | 383 | lsp/server | follow-up | #164: make request dispatch and workspace ownership explicit |
| `crates/recite-benchmarks/src/memory_profiles/mod.rs` | 373 | benchmarks | cohesive | Maintainer-only profile orchestration |
| `crates/recite-cli/src/commands.rs` | 372 | cli | review | Command orchestration boundary |
| `crates/recite-compiler/src/wire/messagepack/tags.rs` | 371 | compiler/wire | cohesive | #89: encoder tag mirror |
| `crates/recite-godot/src/adapter.rs` | 363 | godot | review | Host adapter lifecycle boundary |
| `crates/recite-core/src/schema/model/mod.rs` | 361 | core/schema | review | Schema module exports and model grouping |
| `crates/recite-cli/src/play/driver.rs` | 350 | cli/play | cohesive | Shared preview driver seam |
| `crates/recite-runtime/src/traversal/availability.rs` | 349 | runtime/traversal | cohesive | Deterministic availability traversal |
| `crates/recite-cli/src/play/tui/mod.rs` | 347 | cli/tui | review | TUI integration boundary |
| `crates/recite-core/src/schema/manifest/raw.rs` | 345 | core/schema | cohesive | Lossless raw manifest model |
| `crates/recite-compiler/src/compile/builder/rows.rs` | 337 | compiler | cohesive | Compiled row construction |
| `crates/recite-compiler/src/pot.rs` | 333 | compiler/localisation | follow-up | #164: consume shared authoring analysis |
| `crates/recite-benchmarks/src/id_metrics.rs` | 332 | benchmarks | cohesive | Maintainer metric calculations |
| `crates/recite-ffi/src/output.rs` | 328 | ffi | follow-up | #135: preserve typed failures to the C boundary |
| `crates/recite-runtime/src/traversal/asset.rs` | 328 | runtime/traversal | cohesive | Asset validation and traversal boundary |
| `crates/recite-cli/src/i18n/messages.rs` | 323 | cli/i18n | follow-up | #166: shared Fluent resource ownership |
| `crates/recite-compiler/src/wire/inspection.rs` | 312 | compiler/wire | review | Structured wire inspection projection |
| `crates/recite-benchmarks/src/project.rs` | 310 | benchmarks | cohesive | Synthetic project model |
| `crates/recite-cli/src/play/plain.rs` | 309 | cli/play | cohesive | Plain preview adapter |
| `crates/recite-cli/src/play/tui/state.rs` | 307 | cli/tui | cohesive | TUI reducer state |
| `crates/recite-compiler/src/compile/builder.rs` | 306 | compiler | cohesive | Compiled asset builder |
| `crates/recite-lsp/src/features/completion/projection.rs` | 296 | lsp/features | review | Completion projection |
| `crates/recite-core/src/schema/manifest/spans.rs` | 295 | core/schema | cohesive | Source-span calculation |
| `crates/recite-lsp/src/features.rs` | 295 | lsp/features | review | Intentional ordered feature lookup |
| `crates/recite-cli/src/play/tui/interaction.rs` | 293 | cli/tui | cohesive | Input-to-intent translation |
| `crates/recite-core/src/diagnostic.rs` | 289 | core/diagnostics | cohesive | Shared structured diagnostic surface |
| `crates/recite-compiler/src/validation/metadata.rs` | 285 | compiler/validation | review | Metadata validation ownership |
| `crates/recite-core/src/schema/manifest/lower/functions.rs` | 285 | core/schema | review | Function declaration lowering |
| `crates/recite-compiler/src/validation/conditions.rs` | 282 | compiler/validation | cohesive | Condition validation |
| `crates/recite-core/src/diagnostic/explanation/validation.rs` | 278 | core/diagnostics | cohesive | Diagnostic explanation catalog |
| `crates/recite-cli/src/play/tui/render/prompt.rs` | 270 | cli/tui | cohesive | Prompt rendering |
| `crates/recite-cli/src/tui/config.rs` | 268 | cli/tui | follow-up | #167: replace manual config locations with OS-aware loading |
| `crates/recite-cli/src/cli_help.rs` | 267 | cli | cohesive | CLI help presentation |
| `crates/recite-godot/src/bindings.rs` | 267 | godot | review | Host binding declarations |
| `crates/recite-cli/src/dialogue_locale/po.rs` | 266 | cli/localisation | follow-up | #166: source-preserving PO ownership |
| `crates/recite-cli/src/runtime_fixture/prompt.rs` | 266 | cli/runtime-fixture | cohesive | Fixture prompt projection |
| `crates/recite-benchmarks/src/runtime.rs` | 265 | benchmarks | cohesive | Runtime benchmark harness |
| `crates/recite-lsp/src/summary/file/collector.rs` | 256 | lsp/summary | review | File summary projection |

## Test and support surfaces

| Path | Lines | Owner | Disposition | Issue/reason |
| --- | ---: | --- | --- | --- |
| `crates/recite-runtime/tests/adapter_conformance/driver.rs` | 1,143 | runtime/tests | cohesive | Shared adapter conformance driver |
| `crates/recite-cli/src/play/tui/render/tests.rs` | 811 | cli/tui/tests | cohesive | Private rendering contract tests |
| `crates/recite-cli/tests/runtime.rs` | 759 | cli/tests | cohesive | Runtime command behavior suite |
| `crates/recite-core/tests/compiled_messagepack.rs` | 693 | core/tests | cohesive | Wire compatibility contract |
| `crates/recite-core/tests/schema_manifest/fingerprint.rs` | 638 | core/tests | cohesive | Canonical fingerprint contract |
| `crates/recite-core/tests/support/mod.rs` | 605 | core/tests | cohesive | Shared model and wire constructors |
| `crates/recite-runtime/tests/adapter_conformance/manifest.rs` | 571 | runtime/tests | cohesive | Adapter manifest contract |
| `crates/recite-compiler/tests/asset.rs` | 561 | compiler/tests | cohesive | Compiled asset behavior suite |
| `crates/recite-lsp/src/tests/support.rs` | 423 | lsp/tests | review | Test support ownership; retain private access where required |
| `crates/recite-lsp/src/tests/project_indexes.rs` | 422 | lsp/tests | review | Private index behavior |
| `crates/recite-core/tests/compiled_model.rs` | 419 | core/tests | cohesive | Compiled model behavior |
| `crates/recite-compiler/tests/asset/tag_surface.rs` | 417 | compiler/tests | cohesive | Wire tag surface |
| `crates/recite-runtime/tests/traversal/localisation.rs` | 411 | runtime/tests | cohesive | Locale traversal contract |
| `crates/recite-parser/tests/parser/lowering.rs` | 406 | parser/tests | cohesive | Lowering behavior suite |
| `crates/recite-compiler/tests/pot_extraction.rs` | 404 | compiler/tests | follow-up | #166: shared localisation extraction fixtures |
| `crates/recite-lsp/src/tests/diagnostics.rs` | 402 | lsp/tests | review | Private diagnostic projection tests |
| `crates/recite-fixturegen/tests/generation.rs` | 395 | fixturegen/tests | cohesive | Deterministic generation contract |
| `crates/recite-runtime/tests/adapter_conformance.rs` | 383 | runtime/tests | cohesive | Adapter conformance entry point |
| `crates/recite-core/tests/schema_manifest/load_valid.rs` | 379 | core/tests | cohesive | Valid manifest coverage |
| `crates/recite-runtime/tests/session_serialization/invalid_snapshots.rs` | 378 | runtime/tests | cohesive | Snapshot failure contract |
| `crates/recite-parser/tests/parser/statements.rs` | 374 | parser/tests | cohesive | Statement parser coverage |
| `crates/recite-cli/tests/watch_stress.rs` | 366 | cli/tests | cohesive | Watch stress harness |
| `crates/recite-runtime/tests/traversal/conditions/choice_conditions.rs` | 365 | runtime/tests | cohesive | Choice condition coverage |
| `crates/recite-lsp/src/tests.rs` | 369 | lsp/tests | cohesive | Module aggregator, not production implementation |
| `crates/recite-lsp/src/tests/code_action/schema_entry.rs` | 357 | lsp/tests | review | Private code-action behavior |
