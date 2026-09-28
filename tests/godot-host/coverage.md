# Godot companion evidence

Run `CARGO_TARGET_DIR=/absolute/disk/path GODOT=/path/to/official-godot-4.6.3 scripts/check-godot-host.sh`
and `CARGO_TARGET_DIR=/absolute/disk/path cargo test -p recite-godot --all-targets`.
The host gate packages the addon into a clean project and runs a Godot editor
import, runtime Node/Resource checks, invalid reimport, and a fresh runtime
process. `H` means exercised through actual Godot classes or signals; `R`
means Rust adapter API only; `I` means the host API makes the operation
impossible. Related coverage does not imply exact manifest execution.

All fresh editor scans use `--quit-after 60 --frame-delay 10` and must succeed
on the first invocation. On the pinned official Godot 4.6.3 build, a fresh
extension-only project with no Recite plugin or dialogue assets faulted after
editor layout with `--quit-after 10`; `--import` also faulted. A longer scan
exited successfully in repeated fresh projects. The stripped native stack is
consistent with [Godot issue #111645](https://github.com/godotengine/godot/issues/111645),
but cannot identify the exact engine function. The host gate does not retry a
failed scan; it retains that scan's log under
`$CARGO_TARGET_DIR/godot-host-diagnostics` for diagnosis.

| Manifest scenario ID | Godot evidence |
| --- | --- |
| `validation_error_invalid_source_fixture` | Unrun in Godot; source compilation belongs to the canonical CLI. |
| `asset_load_or_decode_error_truncated_messagepack` | H: malformed byte import and rejected native reimport retain a structured error. |
| `stale_or_incompatible_asset_error_asset_mismatch_on_advance` | I: Node advance operations have no asset argument; the active loaded asset is immutable. |
| `schema_mismatch_error_on_restore` | H: prompt snapshot restored with the same asset ID and different canonical schema manifest is rejected. |
| `no_active_session_error_after_explicit_end` | H: choice after `end_session`. |
| `session_already_active_error_second_start` | H: second Node start. |
| `unknown_start_block_error_missing_block` | H: unknown Node start block. |
| `invalid_choice_error_unknown_choice_id` | H: unknown Node choice. |
| `unavailable_choice_error_conditioned_choice` | H: false Godot callable and unavailable choice. |
| `stale_choice_error_after_prompt_transition` | H: previously observed choice after blocking effect advances to another prompt. |
| `missing_condition_handler_error_no_registered_handler` | H: missing callable, with no partial output signal batch. |
| `condition_evaluation_error_handler_failure` | H: Godot callable returns `ReciteConditionFailure` with caller detail. |
| `invalid_condition_result_error_wrong_result_type` | H: Godot callable returns string for Boolean condition. |
| `effect_acknowledgement_error_wrong_pending_effect_id` | H: wrong pending effect ID. |
| `rejected_changed_asset_refresh_error_active_session_refresh` | Not applicable: reject-until-end policy is not selected. |
| `reload_for_next_session_only_active_session_import` | H: active and available canonical identities diverge after valid refresh, then converge on next session. |
| `restart_required_active_session_refresh_rejection` | Not applicable: restart-required policy is not selected. |
| `save_load_incompatibility_error_snapshot_format_mismatch` | H: malformed snapshot restore; prompt snapshot roundtrip also checked. |
| `plural_line_structured_metadata_adapter_runner_required` | H: count 2, `fr-FR`, exact ordered attempt and all required provenance fields. |
| `plural_line_reference_driver_source_fallback` | H: same plural fixture with no catalogue retains authored text and explicit fallback arm. |
| `localisation_error_adapter_runner_required` | H: malformed PO load through catalogue Resource returns `localisation_error`. |
| `missing_projection_handler_error_adapter_runner_required` | Not applicable: presentation projection capability is not exposed. |
| `projection_evaluation_error_adapter_runner_required` | Not applicable: presentation projection capability is not exposed. |
| `invalid_projection_result_error_adapter_runner_required` | Not applicable: presentation projection capability is not exposed. |
| `source_fingerprint_freshness_capability_gate` | Unavailable for compiled-only import; no source import visibility is claimed. |
| `schema_fingerprint_freshness_capability_gate` | Unavailable for compiled-only import; no schema input visibility is claimed. |

The same host gate checks persisted compiled and PO Resources, canonical
Godot producer schema export, rejected export retention, ordered reentrant
signals, native importer last-good retention, and a clean packaged addon. It
compares independently staged archives byte for byte and checks that a repeat
package run removes obsolete addon files. The example is extracted from the
archive, builds through its own `recite watch` manifest with a successful
structured completion record, and starts in a fresh Godot project. A replacement
install then removes obsolete addon code, preserves authored dialogue source
and compiled bytes, imports again, and starts the example in a new process.

The informational headless profile on an AMD Ryzen AI 7 350, Linux x86_64,
official Godot 4.6.3, debug GDExtension measured 25 loads in 7,724 µs,
25 start/end output conversions in 3,247 µs, 25 condition/effect routes in
3,799 µs, and 500 inactive process notifications in 16 µs. These are sample
totals from one run, not release budgets or frame-time guarantees. The host
gate prints a new bounded measurement on each run.

The same clean host probe with the packaged release GDExtension measured
25 loads in 1,089 µs, 25 start/end routes in 1,573 µs, 25 condition/effect
routes in 1,491 µs, and 500 inactive notifications in 15 µs. The release
bundle also passed native import, rejected refresh retention, and the
packaged example probe.
