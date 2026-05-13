use recite_compiler::{ValidationReport, validate_source_files};
use recite_core::{
    Block, BlockId, Choice, ChoiceId, ChoiceTarget, Diagnostic, DivertTarget, Line, LineId,
    SourceFile, SourcePosition, SourceSpan, SourceText, Statement,
};
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
    assert_eq!(ambiguous.diagnostics[0].span.file, "dialogue/start.recite");
    assert_eq!(ambiguous.diagnostics[0].span.start.line(), 1);
    assert_eq!(
        ambiguous.diagnostics[0].related[0].span.file,
        "dialogue/other.recite"
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

#[test]
fn diagnostics_are_sorted_by_canonical_source_order() {
    let files = vec![lower(
        "dialogue/order.recite",
        concat!(
            ":: start\n",
            "? choose_path\n",
            "  Choose.\n",
            "  >\n",
            "    Nested missing line ID.\n",
            "  -> missing_target\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE005",
            "RECITE_VALIDATE001",
            "RECITE_VALIDATE007",
        ],
    );
    assert_spans(&report, [(1, 1), (4, 3), (6, 3)]);
}

#[test]
fn validation_is_independent_of_caller_file_order() {
    let first = lower(
        "dialogue/a.recite",
        concat!(":: first default\n", "> shared\n", "  First.\n",),
    );
    let second = lower(
        "dialogue/b.recite",
        concat!(":: second default\n", "? shared\n", "  Second.\n",),
    );

    let forward = validate_source_files(&[first.clone(), second.clone()]);
    let reverse = validate_source_files(&[second, first]);

    assert_eq!(forward, reverse);
    assert_codes(&forward, ["RECITE_VALIDATE006", "RECITE_VALIDATE004"]);
    assert_eq!(
        forward.diagnostics[0].related[0].span.file,
        "dialogue/a.recite"
    );
    assert_eq!(
        forward.diagnostics[1].related[0].span.file,
        "dialogue/a.recite"
    );
}

#[test]
fn line_and_choice_ids_share_one_localisable_namespace() {
    let files = vec![lower(
        "dialogue/shared.recite",
        concat!(
            ":: start default\n",
            "> shared\n",
            "  Line.\n",
            "? shared\n",
            "  Choice.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE004"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(report.diagnostics[0].related[0].span.start.line(), 2);
}

#[test]
fn validates_duplicate_block_ids() {
    let files = vec![lower(
        "dialogue/blocks.recite",
        concat!(
            ":: repeated default\n",
            "> first\n",
            "  First.\n",
            ":: repeated\n",
            "> second\n",
            "  Second.\n",
            "-> repeated\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert_codes(&report, ["RECITE_VALIDATE009"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 4);
    assert_eq!(report.diagnostics[0].related[0].span.start.line(), 1);
}

#[test]
fn validates_invalid_source_spans() {
    let mut source_file = SourceFile::new(
        "dialogue/source.recite",
        vec![
            Block::new(
                BlockId::new("start").expect("valid block ID"),
                vec![
                    Statement::Line(Line::new(
                        Some(LineId::new("line").expect("valid line ID")),
                        SourceText::new("Line.", span_range("dialogue/source.recite", 2, 8, 2, 3)),
                        span("dialogue/zz_wrong.recite", 2, 1),
                    )),
                    Statement::Choice(
                        Choice::new(
                            Some(ChoiceId::new("choice").expect("valid choice ID")),
                            SourceText::new("Choice.", span("dialogue/source.recite", 3, 3)),
                            span("dialogue/source.recite", 3, 1),
                        )
                        .with_target(ChoiceTarget::new(
                            DivertTarget::End,
                            span("dialogue/zz_wrong.recite", 4, 3),
                        )),
                    ),
                ],
                span("dialogue/source.recite", 1, 1),
            )
            .with_default(true),
        ],
    );

    source_file.blocks[0].statements.reverse();
    let report = validate_source_files(&[source_file]);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
        ],
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("span end precedes"))
    );
    assert_eq!(
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.message.contains("span file does not match"))
            .count(),
        2
    );
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

fn span(file: &str, line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        file,
        SourcePosition::new(line, column).expect("valid source position"),
    )
}

fn span_range(
    file: &str,
    start_line: u32,
    start_column: u32,
    end_line: u32,
    end_column: u32,
) -> SourceSpan {
    SourceSpan::new(
        file,
        SourcePosition::new(start_line, start_column).expect("valid source position"),
        Some(SourcePosition::new(end_line, end_column).expect("valid source position")),
    )
}
