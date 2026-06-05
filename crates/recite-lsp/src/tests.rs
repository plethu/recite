mod availability;
mod diagnostics;
mod lifecycle;
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
fn did_open_publishes_schema_backed_semantic_diagnostics() {
    diagnostics::did_open_publishes_schema_backed_semantic_diagnostics();
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
fn malformed_completion_and_hover_params_return_invalid_params() {
    availability::malformed_completion_and_hover_params_return_invalid_params();
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
fn metadata_domain_schema_summary_preserves_available_provenance() {
    project_indexes::metadata_domain_schema_summary_preserves_available_provenance();
}

#[test]
fn stale_change_does_not_bump_snapshot_generation() {
    project_indexes::stale_change_does_not_bump_snapshot_generation();
}

#[test]
fn crlf_and_non_bmp_text_use_utf16_ranges() {
    position::crlf_and_non_bmp_text_use_utf16_ranges();
}
