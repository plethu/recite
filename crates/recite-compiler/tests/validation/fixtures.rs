use super::*;

#[test]
fn valid_source_files_produce_no_validation_diagnostics() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "> intro@f508dc4a6d224219aa7d\n",
                "  Hello.\n",
                "? continue@809fb5fd9c42ca38b940\n",
                "  Continue.\n",
                "  -> dialogue/next.recite::next\n",
            ),
        ),
        lower(
            "dialogue/next.recite",
            concat!(
                ":: next\n",
                "> next_line@11111111111111111111\n",
                "  Next.\n",
            ),
        ),
    ];

    assert!(validate_source_files(&files).is_ok());
}

#[test]
fn valid_fixture_can_be_reused_by_compiler_validation() {
    const FIXTURE: &str = "fixtures/recite/valid/core_language_spike.recite";

    let files = vec![lower_fixture(FIXTURE)];
    let report = validate_source_files(&files);

    assert_diagnostic_snapshot(&report.diagnostics, diagnostic_snapshot_name(FIXTURE));
}

#[test]
fn fixture_snapshot_captures_validation_ordering_related_spans_and_help() {
    const FIXTURE: &str = "fixtures/recite/invalid/compiler_validation_order.recite";

    let files = vec![lower_fixture(FIXTURE)];
    let report = validate_source_files(&files);

    assert_diagnostic_snapshot(&report.diagnostics, diagnostic_snapshot_name(FIXTURE));
}
