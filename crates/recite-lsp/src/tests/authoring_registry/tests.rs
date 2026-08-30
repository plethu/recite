#[test]
fn initialize_advertises_completion_and_hover() {
    super::availability::initialize_advertises_completion_and_hover();
}

#[test]
fn resolves_metadata_array_elements_by_declared_type() {
    super::availability::resolves_metadata_array_elements_by_declared_type();
}

#[test]
fn completes_requires_conditions_and_parameterless_reasons() {
    super::availability::completes_requires_conditions_and_parameterless_reasons();
}

#[test]
fn completes_project_and_schema_authoring_symbols() {
    super::availability::completes_project_and_schema_authoring_symbols();
}

#[test]
fn completes_metadata_domain_values_from_schema_context() {
    super::availability::completes_metadata_domain_values_from_schema_context();
}

#[test]
fn completes_projection_schema_authoring_symbols() {
    super::availability::completes_projection_schema_authoring_symbols();
}

#[test]
fn scopes_projection_schema_authoring_symbols_to_current_projector() {
    super::availability::scopes_projection_schema_authoring_symbols_to_current_projector();
}

#[test]
fn does_not_complete_projection_outputs_in_sibling_objects() {
    super::availability::does_not_complete_projection_outputs_in_sibling_objects();
}

#[test]
fn does_not_complete_projection_projectors_in_sibling_objects() {
    super::availability::does_not_complete_projection_projectors_in_sibling_objects();
}

#[test]
fn completion_ignores_non_metadata_authoring_positions() {
    super::availability::completion_ignores_non_metadata_authoring_positions();
}

#[test]
fn hover_distinguishes_unavailable_and_hidden_choices() {
    super::availability::hover_distinguishes_unavailable_and_hidden_choices();
}

#[test]
fn hover_uses_utf16_positions_after_non_ascii_prefix() {
    super::availability::hover_uses_utf16_positions_after_non_ascii_prefix();
}

#[test]
fn hover_describes_schema_and_project_symbols() {
    super::availability::hover_describes_schema_and_project_symbols();
}

#[test]
fn hover_preserves_choice_reason_clause_resolution() {
    super::availability::hover_preserves_choice_reason_clause_resolution();
}

#[test]
fn hover_resolves_choice_speaker_metadata_before_builtin_speakers() {
    super::availability::hover_resolves_choice_speaker_metadata_before_builtin_speakers();
}

#[test]
fn completes_choice_speaker_metadata_by_schema_type() {
    super::availability::completes_choice_speaker_metadata_by_schema_type();
}

#[test]
fn rejects_builtin_speaker_candidates_for_unrelated_choice_metadata_type() {
    super::availability::rejects_builtin_speaker_candidates_for_unrelated_choice_metadata_type();
}

#[test]
fn completes_registry_and_enum_choice_metadata_values() {
    super::availability::completes_registry_and_enum_choice_metadata_values();
}

#[test]
fn filters_registry_metadata_completion_to_source_symbols() {
    super::availability::filters_registry_metadata_completion_to_source_symbols();
}

#[test]
fn filters_enum_metadata_completion_to_source_symbols() {
    super::availability::filters_enum_metadata_completion_to_source_symbols();
}

#[test]
fn filters_contextual_domain_completion_to_source_symbols() {
    super::availability::filters_contextual_domain_completion_to_source_symbols();
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
    super::availability::malformed_completion_and_hover_params_return_invalid_params();
}

#[test]
fn initialize_advertises_missing_id_code_actions() {
    super::code_action::initialize_advertises_missing_id_code_actions();
}

#[test]
fn quick_fix_inserts_marker_only_line_and_choice_ids() {
    super::code_action::quick_fix_inserts_marker_only_line_and_choice_ids();
}

#[test]
fn quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers() {
    super::code_action::quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers();
}

#[test]
fn quick_fix_freezes_draft_and_plain_label_headers() {
    super::code_action::quick_fix_freezes_draft_and_plain_label_headers();
}

#[test]
fn source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids() {
    super::code_action::source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids(
    );
}

#[test]
fn generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions() {
    super::code_action::generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions(
    );
}

#[test]
fn code_actions_use_utf16_crlf_and_indented_ranges() {
    super::code_action::code_actions_use_utf16_crlf_and_indented_ranges();
}

#[test]
fn existing_and_draft_stem_ids_do_not_receive_missing_id_actions() {
    super::code_action::existing_and_draft_stem_ids_do_not_receive_missing_id_actions();
}

#[test]
fn block_stub_quick_fix_inserts_local_eof_stub() {
    super::code_action::block_stub_quick_fix_inserts_local_eof_stub();
}

#[test]
fn block_stub_quick_fix_targets_unique_external_file() {
    super::code_action::block_stub_quick_fix_targets_unique_external_file();
}

#[test]
fn block_stub_quick_fix_rejects_unresolved_target_and_target_collision() {
    super::code_action::block_stub_quick_fix_rejects_unresolved_target_and_target_collision();
}

#[test]
fn block_stub_quick_fix_rejects_incomplete_block_reference_summary() {
    super::code_action::block_stub_quick_fix_rejects_incomplete_block_reference_summary();
}

#[test]
fn condition_schema_quick_fix_inserts_zero_arg_bool_entry() {
    super::code_action::condition_schema_quick_fix_inserts_zero_arg_bool_entry();
}

#[test]
fn condition_schema_quick_fix_rejects_arguments_and_match_scrutinee() {
    super::code_action::condition_schema_quick_fix_rejects_arguments_and_match_scrutinee();
}

#[test]
fn effect_schema_quick_fix_inserts_zero_arg_mode_entry() {
    super::code_action::effect_schema_quick_fix_inserts_zero_arg_mode_entry();
}

#[test]
fn effect_schema_quick_fix_rejects_arguments_and_metadata() {
    super::code_action::effect_schema_quick_fix_rejects_arguments_and_metadata();
}

#[test]
fn schema_entry_quick_fix_uses_project_wide_same_name_function_context() {
    super::code_action::schema_entry_quick_fix_uses_project_wide_same_name_function_context();
}

#[test]
fn schema_entry_quick_fix_rejects_incomplete_project_function_summaries() {
    super::code_action::schema_entry_quick_fix_rejects_incomplete_project_function_summaries();
}

#[test]
fn schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline() {
    super::code_action::schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline();
}

#[test]
fn schema_entry_quick_fix_rejects_missing_sections() {
    super::code_action::schema_entry_quick_fix_rejects_missing_sections();
}

#[test]
fn schema_entry_quick_fix_rejects_open_schema_buffers() {
    super::code_action::schema_entry_quick_fix_rejects_open_schema_buffers();
}

#[test]
fn malformed_code_action_params_return_invalid_params() {
    super::code_action::malformed_code_action_params_return_invalid_params();
}

#[test]
fn definition_resolves_block_references() {
    super::navigation::definition_resolves_block_references();
}

#[test]
fn typed_features_follow_open_overlay_generation() {
    super::navigation::typed_features_follow_open_overlay_generation();
}

#[test]
fn references_include_declaration_and_project_references() {
    super::navigation::references_include_declaration_and_project_references();
}

#[test]
fn rename_updates_only_block_symbols() {
    super::navigation::rename_updates_only_block_symbols();
}

#[test]
fn rename_rejects_non_block_symbols_and_invalid_names() {
    super::navigation::rename_rejects_non_block_symbols_and_invalid_names();
}

#[test]
fn rename_rejects_local_and_qualified_block_collisions() {
    super::navigation_corrections::rename_rejects_local_and_qualified_block_collisions();
}

#[test]
fn references_require_unique_navigation() {
    super::navigation_corrections::references_require_unique_navigation();
}

#[test]
fn typed_clause_and_schema_ranges_exclude_delimiters() {
    super::navigation_corrections::typed_clause_and_schema_ranges_exclude_delimiters();
}

#[test]
fn condition_marker_completion_and_hover_follow_parser_boundaries() {
    super::navigation_corrections::condition_marker_completion_and_hover_follow_parser_boundaries();
}
