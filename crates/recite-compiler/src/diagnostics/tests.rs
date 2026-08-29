use std::collections::BTreeMap;

use recite_core::{Diagnostic, DiagnosticArgumentValue, SourcePosition, SourceSpan};

use super::{
    InterpolationError, PluralError, SourceSpanError, SourceSpanOwner, UnbalancedMarkupKind,
    invalid_interpolation, invalid_plural_line, invalid_source_span, unbalanced_markup_tag,
};

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("test source position is valid")
}

fn point() -> SourceSpan {
    SourceSpan::point("dialogue/start.recite", position(1, 1))
}

fn assert_presentation(
    diagnostic: &Diagnostic,
    expected_id: &str,
    expected_arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) {
    let presentation = diagnostic.presentation.as_ref().expect("presentation");
    assert_eq!(presentation.id().as_str(), expected_id);
    let expected = expected_arguments
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(presentation.arguments(), &expected);
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic.record().expect("diagnostic is recordable");
}

#[test]
fn interpolation_factories_keep_each_variant_exactly_typed() {
    let variants = [
        (
            InterpolationError::Unterminated,
            "diagnostic-validate-045-unterminated",
            Vec::new(),
        ),
        (
            InterpolationError::UnescapedClosingBrace,
            "diagnostic-validate-045-unescaped",
            Vec::new(),
        ),
        (
            InterpolationError::InvalidName("bad-name".to_owned()),
            "diagnostic-validate-045-invalid-name",
            vec![(
                "key",
                DiagnosticArgumentValue::String("bad-name".to_owned()),
            )],
        ),
        (
            InterpolationError::Duplicate("duplicate".to_owned()),
            "diagnostic-validate-045-duplicate",
            vec![(
                "key",
                DiagnosticArgumentValue::String("duplicate".to_owned()),
            )],
        ),
        (
            InterpolationError::Unused("unused".to_owned()),
            "diagnostic-validate-045-unused",
            vec![("key", DiagnosticArgumentValue::String("unused".to_owned()))],
        ),
        (
            InterpolationError::Unbound("unbound".to_owned()),
            "diagnostic-validate-045-unbound",
            vec![("key", DiagnosticArgumentValue::String("unbound".to_owned()))],
        ),
    ];

    for (error, presentation_id, arguments) in variants {
        let diagnostic = invalid_interpolation(point(), error);
        assert_presentation(&diagnostic, presentation_id, arguments);
    }
}

#[test]
fn plural_factories_keep_each_variant_exactly_typed() {
    let variants = [
        (PluralError::Newline, "diagnostic-validate-046-newline"),
        (
            PluralError::MissingCount,
            "diagnostic-validate-046-missing-count",
        ),
        (PluralError::CountType, "diagnostic-validate-046-count-type"),
    ];

    for (error, presentation_id) in variants {
        let diagnostic = invalid_plural_line(point(), error);
        assert_presentation(&diagnostic, presentation_id, []);
    }
}

#[test]
fn bracket_markup_factory_has_no_unbounded_reason_argument() {
    let diagnostic = unbalanced_markup_tag(
        "bold",
        point(),
        UnbalancedMarkupKind::MissingClosingBracket,
        None,
    );
    assert_presentation(&diagnostic, "diagnostic-validate-023-bracket", []);
}

#[test]
fn metadata_span_owners_preserve_both_validate008_shapes() {
    let owners = [
        (SourceSpanOwner::MetadataEntry, "metadata-entry"),
        (SourceSpanOwner::MetadataKey, "metadata-key"),
        (SourceSpanOwner::MetadataValue, "metadata-value"),
    ];
    for (owner, token) in owners {
        let mismatch = invalid_source_span(
            SourceSpan::point("dialogue/other.recite", position(1, 1)),
            owner,
            SourceSpanError::FileMismatch,
        );
        assert_presentation(
            &mismatch,
            "diagnostic-validate-008-file",
            [("owner", DiagnosticArgumentValue::String(token.to_owned()))],
        );

        let reversed = invalid_source_span(
            SourceSpan::new(
                "dialogue/start.recite",
                position(2, 4),
                Some(position(1, 2)),
            ),
            owner,
            SourceSpanError::EndPrecedesStart,
        );
        assert_presentation(
            &reversed,
            "diagnostic-validate-008-order",
            [("owner", DiagnosticArgumentValue::String(token.to_owned()))],
        );
    }
}
