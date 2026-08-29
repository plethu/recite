#[test]
fn malformed_user_config_warns_without_blocking_initialize() {
    super::lifecycle::malformed_user_config_warns_without_blocking_initialize();
}

#[test]
fn explicit_missing_user_config_warns_with_stable_code() {
    super::lifecycle::explicit_missing_user_config_warns_with_stable_code();
}

#[test]
fn translated_config_warning_uses_exact_code_and_detail_once() {
    super::lifecycle::translated_config_warning_uses_exact_code_and_detail_once();
}

#[test]
fn did_open_publishes_source_diagnostics_with_stable_shape() {
    super::diagnostics::did_open_publishes_source_diagnostics_with_stable_shape();
}

#[test]
fn did_open_publishes_lowering_diagnostics() {
    super::diagnostics::did_open_publishes_lowering_diagnostics();
}

#[test]
fn did_open_publishes_schema_less_semantic_diagnostics() {
    super::diagnostics::did_open_publishes_schema_less_semantic_diagnostics();
}

#[test]
fn shared_language_pressure_fixture_publishes_no_diagnostics() {
    super::diagnostics::shared_language_pressure_fixture_publishes_no_diagnostics();
}

#[test]
fn shared_language_pressure_fixture_projects_marker_diagnostics() {
    super::diagnostics::shared_language_pressure_fixture_projects_marker_diagnostics();
}

#[test]
fn did_open_publishes_schema_backed_semantic_diagnostics() {
    super::diagnostics::did_open_publishes_schema_backed_semantic_diagnostics();
}

#[test]
fn related_spans_resolve_project_files_and_target_text() {
    super::diagnostics::related_spans_resolve_project_files_and_target_text();
}

#[test]
fn did_save_publishes_schema_backed_diagnostics_for_closed_project_files() {
    super::diagnostics::did_save_publishes_schema_backed_diagnostics_for_closed_project_files();
}

#[test]
fn did_save_schema_reloads_and_republishes_source_diagnostics() {
    super::diagnostics::did_save_schema_reloads_and_republishes_source_diagnostics();
}

#[test]
fn did_save_schema_reloads_from_non_canonical_schema_uri() {
    super::diagnostics::did_save_schema_reloads_from_non_canonical_schema_uri();
}

#[test]
fn publishes_choice_availability_parser_diagnostics() {
    super::availability::publishes_choice_availability_parser_diagnostics();
}

#[test]
fn publishes_choice_availability_schema_diagnostics() {
    super::availability::publishes_choice_availability_schema_diagnostics();
}

#[test]
fn schema_diagnostics_validate_live_project_before_filtering_to_uri() {
    super::availability::schema_diagnostics_validate_live_project_before_filtering_to_uri();
}

#[test]
fn schema_diagnostics_republish_open_references_after_target_changes() {
    super::availability::schema_diagnostics_republish_open_references_after_target_changes();
}

#[test]
fn manifest_discovery_uses_shared_source_roots() {
    super::project_indexes::manifest_discovery_uses_shared_source_roots();
}

#[test]
fn malformed_manifest_does_not_fall_back_to_saved_walker() {
    super::project_indexes::malformed_manifest_does_not_fall_back_to_saved_walker();
}

#[test]
fn manifest_refresh_is_atomic_and_preserves_open_overlay() {
    super::project_indexes::manifest_refresh_is_atomic_and_preserves_open_overlay();
    super::project_indexes::discovery_transitions::all();
}

#[cfg(unix)]
#[test]
fn saved_uri_replacement_removes_old_canonical_entry() {
    super::project_indexes::saved_uri_replacement_removes_old_canonical_entry();
}

#[test]
fn watched_files_refresh_saved_index_for_create_and_delete() {
    super::project_indexes::watched_files_refresh_saved_index_for_create_and_delete();
}

#[test]
fn saved_project_discovery_is_deterministically_sorted() {
    super::project_indexes::saved_project_discovery_is_deterministically_sorted();
}

#[test]
fn open_summary_overlays_saved_project_summary() {
    super::project_indexes::open_summary_overlays_saved_project_summary();
}

#[test]
fn did_save_rekeys_new_open_file_without_duplicate_summary() {
    super::project_indexes::did_save_rekeys_new_open_file_without_duplicate_summary();
}

#[test]
fn did_save_refreshes_saved_summary_for_closed_files() {
    super::project_indexes::did_save_refreshes_saved_summary_for_closed_files();
}

#[test]
fn did_close_refreshes_saved_summary_before_falling_back() {
    super::project_indexes::did_close_refreshes_saved_summary_before_falling_back();
}

#[test]
fn schema_load_failure_keeps_source_only_snapshot() {
    super::project_indexes::schema_load_failure_keeps_source_only_snapshot();
}

#[test]
fn initialized_publishes_schema_load_diagnostics() {
    super::project_indexes::initialized_publishes_schema_load_diagnostics();
}

#[test]
fn schema_projection_diagnostics_publish_and_clear_after_save() {
    super::project_indexes::schema_projection_diagnostics_publish_and_clear_after_save();
}

#[test]
fn metadata_domain_schema_summary_preserves_available_provenance() {
    super::project_indexes::metadata_domain_schema_summary_preserves_available_provenance();
}

#[test]
fn projection_schema_summary_exposes_queries_projectors_and_labels() {
    super::project_indexes::projection_schema_summary_exposes_queries_projectors_and_labels();
}

#[test]
fn stale_change_does_not_bump_snapshot_generation() {
    super::project_indexes::stale_change_does_not_bump_snapshot_generation();
}
