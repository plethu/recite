# Bevy conformance observations

The published v1 manifest has 26 scenarios. Bevy uses the shared `SessionDriver` in
`transactional_drained_batch` mode: a Start, choice, or acknowledgement returns one ordered event
batch and an error publishes no partial batch. The reference driver observes individual `advance`
steps. The tests compare equivalent host-visible semantics and published category codes; they do not
claim byte-identical step traces. `App` means an actual Bevy `App` with `RecitePlugin`; `direct`
means the native asset/catalogue boundary outside a schedule. Optional projection capability is
unsupported.

| Published scenario ID                                         | Bevy observation                                                                                                                                                                         |
| ------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `validation_error_invalid_source_fixture`                     | Authoring/compiler gate; unrun through shipping Bevy adapter, which accepts compiled bytes only.                                                                                         |
| `asset_load_or_decode_error_truncated_messagepack`            | Direct `ReciteDialogueAsset::from_bytes` rejection with published category; native AssetServer failed-refresh retention also App-tested.                                                 |
| `stale_or_incompatible_asset_error_asset_mismatch_on_advance` | Invariant-only: single owner captures immutable asset, so caller cannot substitute one on advance. Active revision retention and incompatible restore are App-tested.                    |
| `schema_mismatch_error_on_restore`                            | App-tested with two compiled revisions carrying different schema fingerprints; published category.                                                                                       |
| `no_active_session_error_after_explicit_end`                  | App-tested after End; published category.                                                                                                                                                |
| `session_already_active_error_second_start`                   | App-tested; published category, including precedence over handle lookup.                                                                                                                 |
| `unknown_start_block_error_missing_block`                     | App-tested; published category.                                                                                                                                                          |
| `invalid_choice_error_unknown_choice_id`                      | App-tested; published category.                                                                                                                                                          |
| `unavailable_choice_error_conditioned_choice`                 | App-tested category on the published runtime-surface fixture; the separate availability-reasons fixture's detailed reason tree is unrun in Bevy.                                         |
| `stale_choice_error_after_prompt_transition`                  | App-tested after blocking acknowledgement and next prompt; published category. Shared driver regression tests encoder rollback and restore.                                              |
| `missing_condition_handler_error_no_registered_handler`       | App-tested initial transaction; published category and no active session committed.                                                                                                      |
| `condition_evaluation_error_handler_failure`                  | App-tested initial transaction; published category and no active session committed.                                                                                                      |
| `invalid_condition_result_error_wrong_result_type`            | App-tested with enum variant where bool is required; published category and no active session committed.                                                                                 |
| `effect_acknowledgement_error_wrong_pending_effect_id`        | App-tested during blocking effect; published category.                                                                                                                                   |
| `rejected_changed_asset_refresh_error_active_session_refresh` | Gated: nonselected reject-on-refresh policy.                                                                                                                                             |
| `reload_for_next_session_only_active_session_import`          | App-tested with changed compiled content under one Bevy handle: active old revision, next session new revision. Uses an equivalent test source, not the exact published changed fixture. |
| `restart_required_active_session_refresh_rejection`           | Gated: nonselected restart-required policy.                                                                                                                                              |
| `save_load_incompatibility_error_snapshot_format_mismatch`    | App-tested restore rejection with malformed snapshot bytes and published category; exact version-99 mutation is unrun.                                                                   |
| `plural_line_structured_metadata_adapter_runner_required`     | Exact published fixture and expectation through App; compares every ordered plural attempt, source forms, chosen arm, and resolution provenance.                                         |
| `plural_line_reference_driver_source_fallback`                | Exact published fixture/expected source text through App with no catalogue.                                                                                                              |
| `localisation_error_adapter_runner_required`                  | Direct shared catalogue import with malformed plural rule; published category. App catalogue replacement/transactional PO import separately tested.                                      |
| `missing_projection_handler_error_adapter_runner_required`    | Gated: optional presentation projection unsupported.                                                                                                                                     |
| `projection_evaluation_error_adapter_runner_required`         | Gated: optional presentation projection unsupported.                                                                                                                                     |
| `invalid_projection_result_error_adapter_runner_required`     | Gated: optional presentation projection unsupported.                                                                                                                                     |
| `source_fingerprint_freshness_capability_gate`                | Gated in compiled-only runtime: freshness is `Unavailable`; authoring uses `recite check-fresh` with source visible.                                                                     |
| `schema_fingerprint_freshness_capability_gate`                | Gated in compiled-only runtime: freshness is `Unavailable`; authoring uses `recite check-fresh` with schema visible.                                                                     |

The current Bevy suite is therefore a policy-specific host observation suite, not a claim that all
26 reference traces ran byte-for-byte in Bevy. The unrun detailed availability-reason and exact
snapshot mutation shapes remain visible here for a later fixture-runner expansion.
