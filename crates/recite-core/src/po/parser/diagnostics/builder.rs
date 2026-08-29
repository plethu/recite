use std::ops::Range;

use super::{PoDiagnostic, PoHeaderDiagnostic, PoPluralDiagnostic};
use crate::po::parser::types::{PoFieldTarget, PoParseError};
use crate::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticExplanation, SourcePosition, SourceSpan,
};

pub(super) fn string(value: impl Into<String>) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.into())
}

pub(super) fn integer(value: usize) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::Integer(i64::try_from(value).unwrap_or(i64::MAX))
}

pub(super) fn field_name(target: PoFieldTarget) -> String {
    match target {
        PoFieldTarget::Context => "msgctxt".to_owned(),
        PoFieldTarget::SourceText => "msgid".to_owned(),
        PoFieldTarget::PluralSourceText => "msgid_plural".to_owned(),
        PoFieldTarget::Translation => "msgstr".to_owned(),
        PoFieldTarget::PluralTranslation(index) => format!("msgstr[{index}]"),
        PoFieldTarget::Previous(field) => format!("previous {field:?}"),
        PoFieldTarget::Unknown => "unknown".to_owned(),
    }
}

pub(super) fn plural_message(cause: &PoPluralDiagnostic) -> String {
    match cause {
        PoPluralDiagnostic::ContiguousArms => {
            "plural entries require contiguous msgstr[N] arms".to_owned()
        }
        PoPluralDiagnostic::ExpectedArm(expected) => format!("expected msgstr[{expected}]"),
        PoPluralDiagnostic::RequiresPluralSource => "msgstr[N] requires msgid_plural".to_owned(),
        PoPluralDiagnostic::Count { expected, actual } => {
            format!("header declares {expected} plural arms but entry has {actual}")
        }
        PoPluralDiagnostic::InvalidArm(keyword) => format!("invalid plural arm `{keyword}`"),
    }
}

pub(super) fn header_message(cause: &PoHeaderDiagnostic) -> String {
    match cause {
        PoHeaderDiagnostic::MultipleHeaders => {
            "PO document contains multiple header records".to_owned()
        }
        PoHeaderDiagnostic::MissingColon(line) => format!("header line `{line}` lacks `:`"),
        PoHeaderDiagnostic::DuplicateOrEmpty(key) => {
            format!("duplicate or empty header `{key}`")
        }
        PoHeaderDiagnostic::InvalidPluralForms => {
            "Plural-Forms must declare positive nplurals and a plural expression".to_owned()
        }
        PoHeaderDiagnostic::InvalidPluralRule(reason) => {
            format!("Plural-Forms rule is unusable: {reason}")
        }
        PoHeaderDiagnostic::PluralHeaderRequired => {
            "active plural entries require Plural-Forms with nplurals and plural".to_owned()
        }
    }
}

#[allow(
    clippy::expect_used,
    reason = "PO diagnostic selectors and their contracts are private and exhaustive"
)]
fn diagnostic_for(span: SourceSpan, cause: PoDiagnostic) -> PoParseError {
    let code = cause.code();
    let presentation_id = cause.presentation_id();
    let contract = crate::contract_for(&code, &presentation_id)
        .expect("PO diagnostic presentation contract is registered");
    let message = cause.message();
    let kind = cause.kind();
    let diagnostic =
        Diagnostic::error_from_contract(contract, message, span.clone(), cause.arguments())
            .expect("PO diagnostic arguments match their presentation contract")
            .with_explanation_presentation(
                crate::explain_diagnostic_code(&code)
                    .map(DiagnosticExplanation::presentation)
                    .expect("PO diagnostic explanation is registered"),
            );
    PoParseError {
        diagnostic: Box::new(diagnostic),
        kind,
        line: span.start.line() as usize,
        column: span.start.column() as usize,
    }
}

pub(crate) fn error(name: &str, line: usize, cause: PoDiagnostic) -> PoParseError {
    let position = SourcePosition::new(u32::try_from(line).unwrap_or(u32::MAX), 1)
        .unwrap_or_else(|_| SourcePosition::new(1, 1).unwrap_or_else(|_| unreachable!()));
    diagnostic_for(SourceSpan::point(name, position), cause)
}

pub(crate) fn error_span(
    name: &str,
    source: &str,
    range: Range<usize>,
    cause: PoDiagnostic,
) -> PoParseError {
    let start = crate::source_location::position_for_byte_offset(source, range.start);
    let end = crate::source_location::position_for_byte_offset(source, range.end);
    diagnostic_for(SourceSpan::new(name, start, Some(end)), cause)
}
