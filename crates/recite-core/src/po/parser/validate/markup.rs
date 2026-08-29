use super::super::diagnostics::{MarkupDiagnostic, PoDiagnostic, error_span};
use super::super::types::{EntryBuilder, PoFieldTarget};
use crate::MarkupTranslationError;

pub(super) fn validate_translation_markup(
    name: &str,
    source: &str,
    _line: usize,
    b: &EntryBuilder,
    target: PoFieldTarget,
    source_text: &str,
    translation: &str,
) -> Result<(), super::super::PoParseError> {
    if translation.is_empty() {
        return Ok(());
    }
    let cause = match crate::validate_markup_translation(source_text, translation) {
        Ok(()) => return Ok(()),
        Err(MarkupTranslationError::NewTag { tag, .. }) => {
            PoDiagnostic::Markup(MarkupDiagnostic::UnknownTag(tag))
        }
        Err(MarkupTranslationError::MissingTag { tag }) => {
            PoDiagnostic::Markup(MarkupDiagnostic::MissingTag(tag))
        }
        Err(MarkupTranslationError::UnbalancedTag { tag, kind, .. }) => {
            PoDiagnostic::Markup(MarkupDiagnostic::UnbalancedTag { tag, kind })
        }
        Err(MarkupTranslationError::AttributeChange {
            tag,
            expected,
            actual,
            ..
        }) => PoDiagnostic::Markup(MarkupDiagnostic::AttributeChange {
            tag,
            expected,
            actual,
        }),
    };
    let range = b
        .fields
        .iter()
        .find(|field| field.target == target)
        .map(|field| field.value_range.clone())
        .unwrap_or_else(|| 0..0);
    Err(error_span(name, source, range, cause))
}
