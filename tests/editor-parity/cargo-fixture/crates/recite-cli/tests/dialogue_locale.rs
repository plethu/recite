#[path = "dialogue_locale/shared_pressure.rs"]
mod shared_pressure;

#[test]
fn dialogue_locale_falls_back_through_intermediate_locale() {}

#[test]
fn dialogue_locale_falls_back_to_language_catalog() {}

#[test]
fn plural_dialogue_uses_gettext_arms_and_records_resolution_trace() {}

#[test]
fn plural_trace_records_no_match_and_all_empty_source_fallbacks() {}

#[test]
fn run_trace_and_play_plain_preview_dialogue_locale_catalogs() {}
