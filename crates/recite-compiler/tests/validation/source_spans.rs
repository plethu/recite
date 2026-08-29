use std::collections::BTreeMap;

use recite_core::{Diagnostic, DiagnosticArgumentValue};

use super::*;

fn assert_presentation(diagnostic: &Diagnostic, presentation_id: &str, owner: &str) {
    let presentation = diagnostic
        .presentation
        .as_ref()
        .expect("source-span diagnostic presentation");
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(
        presentation.arguments(),
        &BTreeMap::from([(
            "owner".to_owned(),
            DiagnosticArgumentValue::String(owner.to_owned()),
        )])
    );
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic
        .record()
        .expect("source-span diagnostic is recordable");
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
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| {
                diagnostic
                    .presentation
                    .as_ref()
                    .expect("source-span presentation")
                    .id()
                    .as_str()
            })
            .collect::<Vec<_>>(),
        [
            "diagnostic-validate-008-order",
            "diagnostic-validate-008-file",
            "diagnostic-validate-008-file",
        ]
    );
    assert_presentation(
        &report.diagnostics[0],
        "diagnostic-validate-008-order",
        "line-source-text",
    );
    assert_presentation(
        &report.diagnostics[1],
        "diagnostic-validate-008-file",
        "line",
    );
    assert_presentation(
        &report.diagnostics[2],
        "diagnostic-validate-008-file",
        "choice-target",
    );
}

#[test]
fn validates_each_metadata_span_owner_for_file_and_order_errors() {
    let source_file = SourceFile::new(
        "dialogue/source.recite",
        vec![
            Block::new(
                BlockId::new("start").expect("valid block ID"),
                Vec::new(),
                span("dialogue/source.recite", 1, 1),
            )
            .with_default(true)
            .with_metadata(SourceMetadata::from_entries(vec![
                SourceMetadataEntry::new("entry_mismatch", SourceMetadataScalar::Bool(true))
                    .with_source_span(span("dialogue/other.recite", 1, 1)),
                SourceMetadataEntry::new("entry_reversed", SourceMetadataScalar::Bool(true))
                    .with_source_span(span_range("dialogue/source.recite", 2, 2, 1, 1)),
                SourceMetadataEntry::new("key_mismatch", SourceMetadataScalar::Bool(true))
                    .with_key_value_spans(span("dialogue/other.recite", 1, 1), None),
                SourceMetadataEntry::new("key_reversed", SourceMetadataScalar::Bool(true))
                    .with_key_value_spans(span_range("dialogue/source.recite", 2, 2, 1, 1), None),
                SourceMetadataEntry::new("value_mismatch", SourceMetadataScalar::Bool(true))
                    .with_key_value_spans(
                        span("dialogue/source.recite", 1, 1),
                        Some(span("dialogue/other.recite", 1, 1)),
                    ),
                SourceMetadataEntry::new("value_reversed", SourceMetadataScalar::Bool(true))
                    .with_key_value_spans(
                        span("dialogue/source.recite", 1, 1),
                        Some(span_range("dialogue/source.recite", 2, 2, 1, 1)),
                    ),
            ])),
        ],
    );

    let report = validate_source_files(&[source_file]);
    assert_codes(
        &report,
        [
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
            "RECITE_VALIDATE008",
        ],
    );
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| (
                diagnostic
                    .presentation
                    .as_ref()
                    .expect("structured presentation")
                    .id()
                    .as_str(),
                match diagnostic
                    .presentation
                    .as_ref()
                    .expect("structured presentation")
                    .arguments()
                    .get("owner")
                    .expect("source-span owner")
                {
                    DiagnosticArgumentValue::String(value) => value.as_str(),
                    value => panic!("unexpected source-span owner argument: {value:?}"),
                }
            ))
            .collect::<Vec<_>>(),
        [
            ("diagnostic-validate-008-file", "metadata-entry"),
            ("diagnostic-validate-008-file", "metadata-key"),
            ("diagnostic-validate-008-file", "metadata-value"),
            ("diagnostic-validate-008-order", "metadata-entry"),
            ("diagnostic-validate-008-order", "metadata-key"),
            ("diagnostic-validate-008-order", "metadata-value"),
        ]
    );
    for (diagnostic, (presentation_id, owner)) in report.diagnostics.iter().zip([
        ("diagnostic-validate-008-file", "metadata-entry"),
        ("diagnostic-validate-008-file", "metadata-key"),
        ("diagnostic-validate-008-file", "metadata-value"),
        ("diagnostic-validate-008-order", "metadata-entry"),
        ("diagnostic-validate-008-order", "metadata-key"),
        ("diagnostic-validate-008-order", "metadata-value"),
    ]) {
        assert_presentation(diagnostic, presentation_id, owner);
    }
}
