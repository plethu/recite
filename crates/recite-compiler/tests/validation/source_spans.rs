use super::*;

#[test]
fn validates_invalid_source_spans() {
    let mut source_file = SourceFile::new(
        "dialogue/source.recite",
        vec![
            Block::new(
                BlockId::new("start").expect("valid block ID"),
                vec![
                    Statement::Line(Line::new(
                        Some(LineId::new("11111111111111111111").expect("valid line ID")),
                        SourceText::new("Line.", span_range("dialogue/source.recite", 2, 8, 2, 3)),
                        span("dialogue/zz_wrong.recite", 2, 1),
                    )),
                    Statement::Choice(
                        Choice::new(
                            Some(ChoiceId::new("22222222222222222222").expect("valid choice ID")),
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
