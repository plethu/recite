#[test]
fn initialize_advertises_full_sync_save_and_utf16() {
    super::lifecycle::initialize_advertises_full_sync_save_and_utf16();
}

#[test]
fn initialize_defaults_to_utf16_when_client_lists_only_utf8() {
    super::lifecycle::initialize_defaults_to_utf16_when_client_lists_only_utf8();
}

#[test]
fn did_save_without_project_state_is_an_explicit_no_op() {
    super::lifecycle::did_save_without_project_state_is_an_explicit_no_op();
}

#[test]
fn shutdown_request_and_exit_notification_terminate_loop() {
    super::lifecycle::shutdown_request_and_exit_notification_terminate_loop();
}

#[test]
fn exit_before_shutdown_terminates_with_error() {
    super::lifecycle::exit_before_shutdown_terminates_with_error();
}

#[test]
fn valid_ui_config_changes_presentation_only() {
    super::lifecycle::valid_ui_config_changes_presentation_only();
}

#[test]
fn absent_platform_default_uses_defaults_without_warning() {
    super::lifecycle::absent_platform_default_uses_defaults_without_warning();
}

#[test]
fn did_close_removes_state_and_clears_diagnostics() {
    super::diagnostics::did_close_removes_state_and_clears_diagnostics();
}

#[test]
fn full_change_replaces_and_clears_diagnostics() {
    super::sync::full_change_replaces_and_clears_diagnostics();
}

#[test]
fn stale_versions_do_not_publish_or_overwrite_newer_text() {
    super::sync::stale_versions_do_not_publish_or_overwrite_newer_text();
}

#[test]
fn non_full_or_malformed_changes_are_ignored() {
    super::sync::non_full_or_malformed_changes_are_ignored();
}

#[test]
fn change_for_unopened_document_is_ignored() {
    super::sync::change_for_unopened_document_is_ignored();
}

#[test]
fn crlf_and_non_bmp_text_use_utf16_ranges() {
    super::position::crlf_and_non_bmp_text_use_utf16_ranges();
}
