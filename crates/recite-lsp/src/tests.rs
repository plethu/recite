mod availability;
mod code_action;
mod diagnostics;
mod lifecycle;
mod navigation;
mod position;
mod project_indexes;
mod support;
mod sync;

#[test]
fn initialize_advertises_full_sync_save_and_utf16() {
    lifecycle::initialize_advertises_full_sync_save_and_utf16();
}

#[test]
fn initialize_advertises_completion_and_hover() {
    availability::initialize_advertises_completion_and_hover();
}

#[test]
fn resolves_metadata_array_elements_by_declared_type() {
    availability::resolves_metadata_array_elements_by_declared_type();
}

#[test]
fn initialize_advertises_missing_id_code_actions() {
    code_action::initialize_advertises_missing_id_code_actions();
}

#[test]
fn initialize_defaults_to_utf16_when_client_lists_only_utf8() {
    lifecycle::initialize_defaults_to_utf16_when_client_lists_only_utf8();
}

#[test]
fn did_save_without_project_state_is_an_explicit_no_op() {
    lifecycle::did_save_without_project_state_is_an_explicit_no_op();
}

#[test]
fn shutdown_request_and_exit_notification_terminate_loop() {
    lifecycle::shutdown_request_and_exit_notification_terminate_loop();
}

#[test]
fn exit_before_shutdown_terminates_with_error() {
    lifecycle::exit_before_shutdown_terminates_with_error();
}

#[test]
fn did_open_publishes_source_diagnostics_with_stable_shape() {
    diagnostics::did_open_publishes_source_diagnostics_with_stable_shape();
}

#[test]
fn did_open_publishes_lowering_diagnostics() {
    diagnostics::did_open_publishes_lowering_diagnostics();
}

#[test]
fn did_open_publishes_schema_less_semantic_diagnostics() {
    diagnostics::did_open_publishes_schema_less_semantic_diagnostics();
}

#[test]
fn shared_language_pressure_fixture_publishes_no_diagnostics() {
    diagnostics::shared_language_pressure_fixture_publishes_no_diagnostics();
}

#[test]
fn shared_language_pressure_fixture_projects_marker_diagnostics() {
    diagnostics::shared_language_pressure_fixture_projects_marker_diagnostics();
}

#[test]
fn did_open_publishes_schema_backed_semantic_diagnostics() {
    diagnostics::did_open_publishes_schema_backed_semantic_diagnostics();
}

#[test]
fn related_spans_resolve_project_files_and_target_text() {
    diagnostics::related_spans_resolve_project_files_and_target_text();
}

#[test]
fn did_save_publishes_schema_backed_diagnostics_for_closed_project_files() {
    diagnostics::did_save_publishes_schema_backed_diagnostics_for_closed_project_files();
}

#[test]
fn did_save_schema_reloads_and_republishes_source_diagnostics() {
    diagnostics::did_save_schema_reloads_and_republishes_source_diagnostics();
}

#[test]
fn did_save_schema_reloads_from_non_canonical_schema_uri() {
    diagnostics::did_save_schema_reloads_from_non_canonical_schema_uri();
}

#[test]
fn publishes_choice_availability_parser_diagnostics() {
    availability::publishes_choice_availability_parser_diagnostics();
}

#[test]
fn publishes_choice_availability_schema_diagnostics() {
    availability::publishes_choice_availability_schema_diagnostics();
}

#[test]
fn schema_diagnostics_validate_live_project_before_filtering_to_uri() {
    availability::schema_diagnostics_validate_live_project_before_filtering_to_uri();
}

#[test]
fn schema_diagnostics_republish_open_references_after_target_changes() {
    availability::schema_diagnostics_republish_open_references_after_target_changes();
}

#[test]
fn completes_requires_conditions_and_parameterless_reasons() {
    availability::completes_requires_conditions_and_parameterless_reasons();
}

#[test]
fn completes_project_and_schema_authoring_symbols() {
    availability::completes_project_and_schema_authoring_symbols();
}

#[test]
fn completes_metadata_domain_values_from_schema_context() {
    availability::completes_metadata_domain_values_from_schema_context();
}

#[test]
fn completes_projection_schema_authoring_symbols() {
    availability::completes_projection_schema_authoring_symbols();
}

#[test]
fn scopes_projection_schema_authoring_symbols_to_current_projector() {
    availability::scopes_projection_schema_authoring_symbols_to_current_projector();
}

#[test]
fn does_not_complete_projection_outputs_in_sibling_objects() {
    availability::does_not_complete_projection_outputs_in_sibling_objects();
}

#[test]
fn does_not_complete_projection_projectors_in_sibling_objects() {
    availability::does_not_complete_projection_projectors_in_sibling_objects();
}

#[test]
fn completion_ignores_non_metadata_authoring_positions() {
    availability::completion_ignores_non_metadata_authoring_positions();
}

#[test]
fn hover_distinguishes_unavailable_and_hidden_choices() {
    availability::hover_distinguishes_unavailable_and_hidden_choices();
}

#[test]
fn hover_uses_utf16_positions_after_non_ascii_prefix() {
    availability::hover_uses_utf16_positions_after_non_ascii_prefix();
}

#[test]
fn hover_describes_schema_and_project_symbols() {
    availability::hover_describes_schema_and_project_symbols();
}

#[test]
fn hover_preserves_choice_reason_clause_resolution() {
    availability::hover_preserves_choice_reason_clause_resolution();
}

#[test]
fn hover_resolves_choice_speaker_metadata_before_builtin_speakers() {
    availability::hover_resolves_choice_speaker_metadata_before_builtin_speakers();
}

#[test]
fn completes_choice_speaker_metadata_by_schema_type() {
    availability::completes_choice_speaker_metadata_by_schema_type();
}

#[test]
fn rejects_builtin_speaker_candidates_for_unrelated_choice_metadata_type() {
    availability::rejects_builtin_speaker_candidates_for_unrelated_choice_metadata_type();
}

#[test]
fn completes_registry_and_enum_choice_metadata_values() {
    availability::completes_registry_and_enum_choice_metadata_values();
}

#[test]
fn filters_registry_metadata_completion_to_source_symbols() {
    availability::filters_registry_metadata_completion_to_source_symbols();
}

#[test]
fn filters_enum_metadata_completion_to_source_symbols() {
    availability::filters_enum_metadata_completion_to_source_symbols();
}

#[test]
fn schema_hover_preserves_freshness_without_producer_identity() {
    let mut schema = recite_core::ProjectSchema::empty_v1();
    schema.producer_metadata = Some(recite_core::ProducerMetadata {
        producer: None,
        content_fingerprint: Some(
            recite_core::producer_content_fingerprint(
                "blake3",
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .expect("valid content fingerprint"),
        ),
        schema_export_version: None,
        inclusion_policy: None,
        producer_fingerprints: Vec::new(),
    });
    let scoped = vec![recite_core::ProducerFingerprint {
        id: "items".to_owned(),
        kind: "fixture".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "items-v1".to_owned(),
    }];
    let catalog =
        recite_ui::UiCatalog::load(&recite_ui::UiLocale::default()).expect("default UI catalog");
    let detail = crate::features::schema_hover::hover_detail(None, &schema, &scoped, &catalog);
    assert!(detail.contains("Content fingerprint blake3:"));
    assert!(detail.contains("items-v1"));
    assert!(!detail.contains("Schema producer"));
}

#[test]
fn schema_hover_uses_choice_selector_site_with_injected_catalog() {
    let localized = recite_ui::DEFAULT_RESOURCE.replace(
        "lsp-hover-schema-scoped-fingerprints =  (scoped: {$fingerprints})",
        "lsp-hover-schema-scoped-fingerprints =  (scope localisé: {$fingerprints})",
    );
    let catalog = recite_ui::UiCatalog::from_resources(
        "fr".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                recite_ui::DEFAULT_RESOURCE.to_owned(),
            ),
            ("fr".parse().expect("locale"), localized),
        ],
    )
    .expect("catalog");
    let schema = recite_core::ProjectSchema::empty_v1();
    let scoped = vec![recite_core::ProducerFingerprint {
        id: "items".to_owned(),
        kind: "fixture".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "items-v1".to_owned(),
    }];
    let detail = crate::features::schema_hover::hover_detail(None, &schema, &scoped, &catalog);
    assert!(detail.contains("scope localisé:"));
    assert!(!detail.contains("(scoped:"));
}

#[test]
fn malformed_completion_and_hover_params_return_invalid_params() {
    availability::malformed_completion_and_hover_params_return_invalid_params();
}

#[test]
fn quick_fix_inserts_marker_only_line_and_choice_ids() {
    code_action::quick_fix_inserts_marker_only_line_and_choice_ids();
}

#[test]
fn quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers() {
    code_action::quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers();
}

#[test]
fn quick_fix_freezes_draft_and_plain_label_headers() {
    code_action::quick_fix_freezes_draft_and_plain_label_headers();
}

#[test]
fn source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids() {
    code_action::source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids();
}

#[test]
fn generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions() {
    code_action::generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions();
}

#[test]
fn code_actions_use_utf16_crlf_and_indented_ranges() {
    code_action::code_actions_use_utf16_crlf_and_indented_ranges();
}

#[test]
fn existing_and_draft_stem_ids_do_not_receive_missing_id_actions() {
    code_action::existing_and_draft_stem_ids_do_not_receive_missing_id_actions();
}

#[test]
fn block_stub_quick_fix_inserts_local_eof_stub() {
    code_action::block_stub_quick_fix_inserts_local_eof_stub();
}

#[test]
fn block_stub_quick_fix_targets_unique_external_file() {
    code_action::block_stub_quick_fix_targets_unique_external_file();
}

#[test]
fn block_stub_quick_fix_rejects_unresolved_target_and_target_collision() {
    code_action::block_stub_quick_fix_rejects_unresolved_target_and_target_collision();
}

#[test]
fn block_stub_quick_fix_rejects_incomplete_block_reference_summary() {
    code_action::block_stub_quick_fix_rejects_incomplete_block_reference_summary();
}

#[test]
fn condition_schema_quick_fix_inserts_zero_arg_bool_entry() {
    code_action::condition_schema_quick_fix_inserts_zero_arg_bool_entry();
}

#[test]
fn condition_schema_quick_fix_rejects_arguments_and_match_scrutinee() {
    code_action::condition_schema_quick_fix_rejects_arguments_and_match_scrutinee();
}

#[test]
fn effect_schema_quick_fix_inserts_zero_arg_mode_entry() {
    code_action::effect_schema_quick_fix_inserts_zero_arg_mode_entry();
}

#[test]
fn effect_schema_quick_fix_rejects_arguments_and_metadata() {
    code_action::effect_schema_quick_fix_rejects_arguments_and_metadata();
}

#[test]
fn schema_entry_quick_fix_uses_project_wide_same_name_function_context() {
    code_action::schema_entry_quick_fix_uses_project_wide_same_name_function_context();
}

#[test]
fn schema_entry_quick_fix_rejects_incomplete_project_function_summaries() {
    code_action::schema_entry_quick_fix_rejects_incomplete_project_function_summaries();
}

#[test]
fn schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline() {
    code_action::schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline();
}

#[test]
fn schema_entry_quick_fix_rejects_missing_sections() {
    code_action::schema_entry_quick_fix_rejects_missing_sections();
}

#[test]
fn schema_entry_quick_fix_rejects_open_schema_buffers() {
    code_action::schema_entry_quick_fix_rejects_open_schema_buffers();
}

#[test]
fn malformed_code_action_params_return_invalid_params() {
    code_action::malformed_code_action_params_return_invalid_params();
}

#[test]
fn definition_resolves_block_references() {
    navigation::definition_resolves_block_references();
}

#[test]
fn references_include_declaration_and_project_references() {
    navigation::references_include_declaration_and_project_references();
}

#[test]
fn rename_updates_only_block_symbols() {
    navigation::rename_updates_only_block_symbols();
}

#[test]
fn rename_rejects_non_block_symbols_and_invalid_names() {
    navigation::rename_rejects_non_block_symbols_and_invalid_names();
}

#[test]
fn did_close_removes_state_and_clears_diagnostics() {
    diagnostics::did_close_removes_state_and_clears_diagnostics();
}

#[test]
fn full_change_replaces_and_clears_diagnostics() {
    sync::full_change_replaces_and_clears_diagnostics();
}

#[test]
fn stale_versions_do_not_publish_or_overwrite_newer_text() {
    sync::stale_versions_do_not_publish_or_overwrite_newer_text();
}

#[test]
fn non_full_or_malformed_changes_are_ignored() {
    sync::non_full_or_malformed_changes_are_ignored();
}

#[test]
fn change_for_unopened_document_is_ignored() {
    sync::change_for_unopened_document_is_ignored();
}

#[test]
fn saved_project_discovery_is_deterministically_sorted() {
    project_indexes::saved_project_discovery_is_deterministically_sorted();
}

#[test]
fn open_summary_overlays_saved_project_summary() {
    project_indexes::open_summary_overlays_saved_project_summary();
}

#[test]
fn did_save_rekeys_new_open_file_without_duplicate_summary() {
    project_indexes::did_save_rekeys_new_open_file_without_duplicate_summary();
}

#[test]
fn did_save_refreshes_saved_summary_for_closed_files() {
    project_indexes::did_save_refreshes_saved_summary_for_closed_files();
}

#[test]
fn did_close_refreshes_saved_summary_before_falling_back() {
    project_indexes::did_close_refreshes_saved_summary_before_falling_back();
}

#[test]
fn schema_load_failure_keeps_source_only_snapshot() {
    project_indexes::schema_load_failure_keeps_source_only_snapshot();
}

#[test]
fn initialized_publishes_schema_load_diagnostics() {
    project_indexes::initialized_publishes_schema_load_diagnostics();
}

#[test]
fn schema_projection_diagnostics_publish_and_clear_after_save() {
    project_indexes::schema_projection_diagnostics_publish_and_clear_after_save();
}

#[test]
fn metadata_domain_schema_summary_preserves_available_provenance() {
    project_indexes::metadata_domain_schema_summary_preserves_available_provenance();
}

#[test]
fn projection_schema_summary_exposes_queries_projectors_and_labels() {
    project_indexes::projection_schema_summary_exposes_queries_projectors_and_labels();
}

#[test]
fn stale_change_does_not_bump_snapshot_generation() {
    project_indexes::stale_change_does_not_bump_snapshot_generation();
}

#[test]
fn crlf_and_non_bmp_text_use_utf16_ranges() {
    position::crlf_and_non_bmp_text_use_utf16_ranges();
}
