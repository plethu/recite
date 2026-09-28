# Unity adapter conformance coverage (v1 manifest)

The manifest is `fixtures/adapter-conformance/v1/scenarios.json` (26 IDs).
The common reference driver executes its `reference_driver` scenarios; the
Unity managed suite observes transactional drained batches through real
`recite-ffi` 0.6.0. It does not claim byte-identical traces with the
reference driver's individual `advance` operations. These mappings identify
actual Unity-side evidence and gaps. `scripts/check-unity-adapter.sh` runs the
headless suite and CLI export; `scripts/unity/run-unity-tests.sh` runs host
EditMode/PlayMode tests when an Editor is available.

| Manifest ID | Unity observation / state |
| --- | --- |
| `validation_error_invalid_source_fixture` | Common reference driver; Unity CLI schema duplicate diagnostic tested by `check-schema-export.sh`, but not this exact source fixture. |
| `asset_load_or_decode_error_truncated_messagepack` | `InspectCanonicalAssetInfo` rejects corrupt compiled bytes as `AssetLoadOrDecode` through native FFI; `RejectedReimportKeepsLastValidRevisionAndRecoveryClearsError` passes in both tested Editors. |
| `stale_or_incompatible_asset_error_asset_mismatch_on_advance` | Not an exposed managed operation: `NativeSession` retains the loaded asset and `SelectChoice`/`AcknowledgeEffect` cannot receive another asset; common reference driver covers explicit mismatch. |
| `schema_mismatch_error_on_restore` | `RestoreRejectsDifferentSchema` compiles the same source/asset path against two canonical CLI-exported manifests with distinct schema fingerprints, then verifies managed/native `SchemaMismatch`, cleanup, and successful original restore. |
| `no_active_session_error_after_explicit_end` | `ManagedContractErrors` verifies `Snapshot` after `End` returns `NoActiveSession`. |
| `session_already_active_error_second_start` | `ManagedTraversalAndRestore` and `RetainActiveRevisionAcrossCandidateUpdate` verify `SessionAlreadyActive` and preserved active revision. |
| `unknown_start_block_error_missing_block` | `ManagedContractErrors` verifies `UnknownStartBlock` and failed-start cleanup. |
| `invalid_choice_error_unknown_choice_id` | `ManagedContractErrors` verifies `InvalidChoice`. |
| `unavailable_choice_error_conditioned_choice` | `ManagedTraversalAndRestore` verifies unavailable structured choice and `UnavailableChoice`; common reference driver covers the exact availability fixture. |
| `stale_choice_error_after_prompt_transition` | `StaleChoiceAfterLaterPrompt` compiles the shared runtime-surface fixture, observes a later prompt, then verifies the prior choice is `StaleChoice` while an unknown ID is `InvalidChoice`. |
| `missing_condition_handler_error_no_registered_handler` | `ManagedContractErrors` verifies `MissingConditionHandler` and cleanup. |
| `condition_evaluation_error_handler_failure` | `ManagedContractErrors` verifies `ConditionEvaluation` from a throwing C# handler and cleanup. |
| `invalid_condition_result_error_wrong_result_type` | `ManagedContractErrors` verifies `InvalidConditionResult` from an enum value returned to a bool condition. |
| `effect_acknowledgement_error_wrong_pending_effect_id` | `ManagedContractErrors` and `ManagedTraversalAndRestore` verify `EffectAcknowledgement` and retained pending request. |
| `rejected_changed_asset_refresh_error_active_session_refresh` | Not selected: Unity policy is `reload_for_next_session_only`; common reference driver tests the alternate policy. |
| `reload_for_next_session_only_active_session_import` | `RetainActiveRevisionAcrossCandidateUpdate` checks old active/new available native fingerprints and next start; `RejectedReimportKeepsLastValidRevisionAndRecoveryClearsError` passes in both tested Editors. |
| `restart_required_active_session_refresh_rejection` | Not selected: alternate policy covered by common reference driver. |
| `save_load_incompatibility_error_snapshot_format_mismatch` | `ManagedContractErrors` rejects corrupt snapshot as `SaveLoadIncompatibility`; `RetainActiveRevisionAcrossCandidateUpdate` rejects same-name changed-payload restore. |
| `plural_line_structured_metadata_adapter_runner_required` | `PreservePluralMetadataFromNativePo` compiles the shared plural fixture and checks translated text, both source forms, count, selected arm, all matched resolution fields, and the exact ordered attempt fields. |
| `plural_line_reference_driver_source_fallback` | `PreservePluralMetadataFromNativePo` checks source fallback text, arm, and provenance with no PO provider. |
| `localisation_error_adapter_runner_required` | `PoLocaleModeTransitions` and `RejectInvalidLocaleStringsAndPluralCounts` verify malformed PO/plural rule returns `Localisation`; failed candidate leaves old catalogue active. |
| `missing_projection_handler_error_adapter_runner_required` | `presentation_projection` capability is not exposed by the Unity package. |
| `projection_evaluation_error_adapter_runner_required` | `presentation_projection` capability is not exposed by the Unity package. |
| `invalid_projection_result_error_adapter_runner_required` | `presentation_projection` capability is not exposed by the Unity package. |
| `source_fingerprint_freshness_capability_gate` | `source_import_visibility` unavailable for an isolated `.recitec` import; no freshness claim. |
| `schema_fingerprint_freshness_capability_gate` | `schema_import_visibility` unavailable for an isolated `.recitec` import; no freshness claim. |

Clean projects installed from the UPM artifact pass all three EditMode and
three PlayMode tests on Unity 2022.3.62f3 and 6000.7.0b2, Linux x86_64. These
cover scene references and migration, rejected import retention, runner
lifecycle, and reentrant event ordering. The imported-resource player test
passes on 2022.3 Mono and 6.7 IL2CPP, and in a separate experimental 6.7
CoreCLR player. Player coverage is that one smoke test; the larger managed
conformance suite above runs through .NET 8 and the native library.

Unity 6.7+ is the primary target; the pinned 2022.3 Mono lane is best effort.
The 6.7 Editor still uses Mono. CoreCLR is experimental and other platforms
remain unverified. Projection is unsupported, and an isolated compiled asset
cannot establish source/schema freshness.
