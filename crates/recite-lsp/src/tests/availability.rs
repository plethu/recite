mod authoring;
mod diagnostics;

pub(super) fn initialize_advertises_completion_and_hover() {
    authoring::initialize_advertises_completion_and_hover();
}

pub(super) fn publishes_choice_availability_parser_diagnostics() {
    diagnostics::publishes_choice_availability_parser_diagnostics();
}

pub(super) fn publishes_choice_availability_schema_diagnostics() {
    diagnostics::publishes_choice_availability_schema_diagnostics();
}

pub(super) fn schema_diagnostics_validate_live_project_before_filtering_to_uri() {
    diagnostics::schema_diagnostics_validate_live_project_before_filtering_to_uri();
}

pub(super) fn schema_diagnostics_republish_open_references_after_target_changes() {
    diagnostics::schema_diagnostics_republish_open_references_after_target_changes();
}

pub(super) fn completes_requires_conditions_and_parameterless_reasons() {
    authoring::completes_requires_conditions_and_parameterless_reasons();
}

pub(super) fn hover_distinguishes_unavailable_and_hidden_choices() {
    authoring::hover_distinguishes_unavailable_and_hidden_choices();
}

pub(super) fn hover_uses_utf16_positions_after_non_ascii_prefix() {
    authoring::hover_uses_utf16_positions_after_non_ascii_prefix();
}

pub(super) fn malformed_completion_and_hover_params_return_invalid_params() {
    authoring::malformed_completion_and_hover_params_return_invalid_params();
}
