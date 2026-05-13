use recite_compiler::{ValidationReport, validate_source_files};
use recite_core::Diagnostic;
use recite_parser::parse;

#[test]
fn valid_source_files_produce_no_validation_diagnostics() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "> intro\n",
                "  Hello.\n",
                "? continue\n",
                "  Continue.\n",
                "  -> dialogue/next.recite::next\n",
            ),
        ),
        lower(
            "dialogue/next.recite",
            concat!(":: next\n", "> next_line\n", "  Next.\n",),
        ),
    ];

    assert!(validate_source_files(&files).is_ok());
}

#[test]
fn validates_missing_and_duplicate_line_and_choice_ids_in_source_order() {
    let files = vec![lower(
        "dialogue/tavern.recite",
        concat!(
            ":: start default\n",
            ">\n",
            "  Missing line id.\n",
            "> repeated_line\n",
            "  First repeated line id.\n",
            "> prompt\n",
            "  Prompt.\n",
            "  ?\n",
            "    Missing choice id.\n",
            "  ? repeated_choice\n",
            "    First repeated choice id.\n",
            "  ? repeated_choice\n",
            "    Second repeated choice id.\n",
            "> repeated_line\n",
            "  Second repeated line id.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE001",
            "RECITE_VALIDATE002",
            "RECITE_VALIDATE004",
            "RECITE_VALIDATE003",
        ],
    );
    assert_spans(&report, [(2, 1), (8, 3), (12, 3), (14, 1)]);
    assert_eq!(report.diagnostics[2].related[0].span.start.line(), 10);
    assert_eq!(report.diagnostics[3].related[0].span.start.line(), 4);
}

#[test]
fn validates_missing_and_ambiguous_default_blocks() {
    let missing = validate_source_files(&[lower(
        "dialogue/no_default.recite",
        concat!(":: start\n", "> intro\n", "  Hello.\n",),
    )]);

    assert_codes(&missing, ["RECITE_VALIDATE005"]);
    assert_eq!(
        missing.diagnostics[0].span.file,
        "dialogue/no_default.recite"
    );
    assert_eq!(missing.diagnostics[0].span.start.line(), 1);

    let ambiguous = validate_source_files(&[
        lower(
            "dialogue/start.recite",
            concat!(":: start default\n", "> intro\n", "  Hello.\n",),
        ),
        lower(
            "dialogue/other.recite",
            concat!(":: other default\n", "> other_line\n", "  Hello.\n",),
        ),
    ]);

    assert_codes(&ambiguous, ["RECITE_VALIDATE006"]);
    assert_eq!(ambiguous.diagnostics[0].span.file, "dialogue/other.recite");
    assert_eq!(ambiguous.diagnostics[0].span.start.line(), 1);
    assert_eq!(
        ambiguous.diagnostics[0].related[0].span.file,
        "dialogue/start.recite"
    );
}

#[test]
fn validates_unknown_block_references_from_diverts_and_choice_targets() {
    let files = vec![
        lower(
            "dialogue/start.recite",
            concat!(
                ":: start default\n",
                "-> later\n",
                "-> missing_local\n",
                "? choose_path\n",
                "  Choose a path.\n",
                "  -> missing_choice_target\n",
                "-> dialogue/next.recite::next\n",
                "-> dialogue/next.recite::missing_external\n",
                ":: later\n",
                "> after\n",
                "  Later.\n",
            ),
        ),
        lower(
            "dialogue/next.recite",
            concat!(":: next\n", "> next_line\n", "  Next.\n",),
        ),
    ];

    let report = validate_source_files(&files);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE007",
            "RECITE_VALIDATE007",
            "RECITE_VALIDATE007",
        ],
    );
    assert_spans(&report, [(3, 1), (6, 3), (8, 1)]);
}

#[test]
fn empty_project_has_no_span_to_report() {
    assert!(validate_source_files(&[]).is_ok());
}

fn lower(path: &str, source: &str) -> recite_core::SourceFile {
    let parse = parse(path, source);
    let lowered = parse.lower_source_file();

    assert!(
        lowered.diagnostics.is_empty(),
        "test fixture must parse/lower cleanly: {:?}",
        lowered.diagnostics
    );

    lowered.source_file
}

fn assert_codes<const N: usize>(report: &ValidationReport, expected: [&str; N]) {
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>(),
        expected
    );
}

fn assert_spans<const N: usize>(report: &ValidationReport, expected: [(u32, u32); N]) {
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(diagnostic_start)
            .collect::<Vec<_>>(),
        expected
    );
}

fn diagnostic_start(diagnostic: &Diagnostic) -> (u32, u32) {
    (diagnostic.span.start.line(), diagnostic.span.start.column())
}
