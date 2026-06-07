mod block_stub;
mod missing_id;
mod schema_entry;
mod support;

pub(super) fn initialize_advertises_missing_id_code_actions() {
    missing_id::initialize_advertises_missing_id_code_actions();
}

pub(super) fn quick_fix_inserts_marker_only_line_and_choice_ids() {
    missing_id::quick_fix_inserts_marker_only_line_and_choice_ids();
}

pub(super) fn quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers() {
    missing_id::quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers();
}

pub(super) fn quick_fix_freezes_draft_and_plain_label_headers() {
    missing_id::quick_fix_freezes_draft_and_plain_label_headers();
}

pub(super) fn source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids() {
    missing_id::source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids();
}

pub(super) fn generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions() {
    missing_id::generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions();
}

pub(super) fn code_actions_use_utf16_crlf_and_indented_ranges() {
    missing_id::code_actions_use_utf16_crlf_and_indented_ranges();
}

pub(super) fn existing_and_draft_stem_ids_do_not_receive_missing_id_actions() {
    missing_id::existing_and_draft_stem_ids_do_not_receive_missing_id_actions();
}

pub(super) fn block_stub_quick_fix_inserts_local_eof_stub() {
    block_stub::block_stub_quick_fix_inserts_local_eof_stub();
}

pub(super) fn block_stub_quick_fix_targets_unique_external_file() {
    block_stub::block_stub_quick_fix_targets_unique_external_file();
}

pub(super) fn block_stub_quick_fix_rejects_unresolved_target_and_target_collision() {
    block_stub::block_stub_quick_fix_rejects_unresolved_target_and_target_collision();
}

pub(super) fn block_stub_quick_fix_rejects_incomplete_block_reference_summary() {
    block_stub::block_stub_quick_fix_rejects_incomplete_block_reference_summary();
}

pub(super) fn condition_schema_quick_fix_inserts_zero_arg_bool_entry() {
    schema_entry::condition_schema_quick_fix_inserts_zero_arg_bool_entry();
}

pub(super) fn condition_schema_quick_fix_rejects_arguments_and_match_scrutinee() {
    schema_entry::condition_schema_quick_fix_rejects_arguments_and_match_scrutinee();
}

pub(super) fn effect_schema_quick_fix_inserts_zero_arg_mode_entry() {
    schema_entry::effect_schema_quick_fix_inserts_zero_arg_mode_entry();
}

pub(super) fn effect_schema_quick_fix_rejects_arguments_and_metadata() {
    schema_entry::effect_schema_quick_fix_rejects_arguments_and_metadata();
}

pub(super) fn schema_entry_quick_fix_uses_project_wide_same_name_function_context() {
    schema_entry::schema_entry_quick_fix_uses_project_wide_same_name_function_context();
}

pub(super) fn schema_entry_quick_fix_rejects_incomplete_project_function_summaries() {
    schema_entry::schema_entry_quick_fix_rejects_incomplete_project_function_summaries();
}

pub(super) fn schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline() {
    schema_entry::schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline();
}

pub(super) fn schema_entry_quick_fix_rejects_missing_sections() {
    schema_entry::schema_entry_quick_fix_rejects_missing_sections();
}

pub(super) fn schema_entry_quick_fix_rejects_open_schema_buffers() {
    schema_entry::schema_entry_quick_fix_rejects_open_schema_buffers();
}

pub(super) fn malformed_code_action_params_return_invalid_params() {
    missing_id::malformed_code_action_params_return_invalid_params();
}
