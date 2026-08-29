use self::builder::{field_name, header_message, integer, plural_message, string};
use super::types::{PoDiagnosticKind, PoFieldTarget};
use crate::{DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, PluralRuleError};

mod builder;
mod markup;

pub(super) use builder::{error, error_span};
pub(super) use markup::MarkupDiagnostic;

const PO_PARSE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE034");
const PO_STABLE_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID034");
const PO_DUPLICATE_KEY: DiagnosticCode = DiagnosticCode::new_static("RECITE_ID035");
const PO_PLACEHOLDER: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE042");
const PO_PLURAL: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE043");
const PO_HEADER: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE044");

/// A typed cause projected into legacy messages and structured presentations.
pub(super) enum PoDiagnostic {
    ExpectedDirective,
    ExpectedQuotedString,
    MissingField(&'static str),
    DuplicateField(PoFieldTarget),
    QuotedContinuationWithoutField,
    UnexpectedTextAfterQuotedString,
    UnterminatedQuotedString,
    UnsupportedEscape(String),
    InvalidStableId(String),
    PlaceholderMismatch(String),
    InvalidPluralArms(PoPluralDiagnostic),
    InvalidHeader(PoHeaderDiagnostic),
    InvalidFieldOrder(PoFieldTarget),
    DuplicateKey {
        context: String,
        source_text: String,
    },
    Markup(MarkupDiagnostic),
}

pub(super) enum PoPluralDiagnostic {
    ContiguousArms,
    ExpectedArm(usize),
    RequiresPluralSource,
    Count { expected: usize, actual: usize },
    InvalidArm(String),
}

pub(super) enum PoHeaderDiagnostic {
    MultipleHeaders,
    MissingColon(String),
    DuplicateOrEmpty(String),
    InvalidPluralForms,
    InvalidPluralRule(PluralRuleError),
    PluralHeaderRequired,
}

impl PoDiagnostic {
    fn code(&self) -> DiagnosticCode {
        match self {
            Self::ExpectedDirective
            | Self::ExpectedQuotedString
            | Self::MissingField(_)
            | Self::DuplicateField(_)
            | Self::QuotedContinuationWithoutField
            | Self::UnexpectedTextAfterQuotedString
            | Self::UnterminatedQuotedString
            | Self::UnsupportedEscape(_)
            | Self::InvalidFieldOrder(_) => PO_PARSE,
            Self::InvalidStableId(_) => PO_STABLE_ID,
            Self::PlaceholderMismatch(_) => PO_PLACEHOLDER,
            Self::InvalidPluralArms(_) => PO_PLURAL,
            Self::InvalidHeader(_) => PO_HEADER,
            Self::DuplicateKey { .. } => PO_DUPLICATE_KEY,
            Self::Markup(cause) => cause.code(),
        }
    }

    fn presentation_id(&self) -> DiagnosticPresentationId {
        if let Self::Markup(cause) = self {
            return cause.presentation_id();
        }
        let value = match self {
            Self::ExpectedDirective => "diagnostic-parse-034-expected-directive",
            Self::ExpectedQuotedString => "diagnostic-parse-034-expected-quoted-string",
            Self::MissingField(_) => "diagnostic-parse-034-missing-field",
            Self::DuplicateField(_) => "diagnostic-parse-034-duplicate-field",
            Self::QuotedContinuationWithoutField => "diagnostic-parse-034-quoted-without-field",
            Self::UnexpectedTextAfterQuotedString => {
                "diagnostic-parse-034-unexpected-trailing-text"
            }
            Self::UnterminatedQuotedString => "diagnostic-parse-034-unterminated-quoted-string",
            Self::UnsupportedEscape(_) => "diagnostic-parse-034-unsupported-escape",
            Self::InvalidFieldOrder(_) => "diagnostic-parse-034-invalid-field-order",
            Self::InvalidStableId(_) => "diagnostic-id-034",
            Self::DuplicateKey { .. } => "diagnostic-id-035",
            Self::PlaceholderMismatch(_) => "diagnostic-validate-042",
            Self::InvalidPluralArms(cause) => match cause {
                PoPluralDiagnostic::ContiguousArms => "diagnostic-validate-043-contiguous-arms",
                PoPluralDiagnostic::ExpectedArm(_) => "diagnostic-validate-043-expected-arm",
                PoPluralDiagnostic::RequiresPluralSource => {
                    "diagnostic-validate-043-requires-plural-source"
                }
                PoPluralDiagnostic::Count { .. } => "diagnostic-validate-043-count",
                PoPluralDiagnostic::InvalidArm(_) => "diagnostic-validate-043-invalid-arm",
            },
            Self::InvalidHeader(cause) => match cause {
                PoHeaderDiagnostic::MultipleHeaders => "diagnostic-validate-044-multiple-headers",
                PoHeaderDiagnostic::MissingColon(_) => "diagnostic-validate-044-missing-colon",
                PoHeaderDiagnostic::DuplicateOrEmpty(_) => {
                    "diagnostic-validate-044-duplicate-or-empty"
                }
                PoHeaderDiagnostic::InvalidPluralForms => {
                    "diagnostic-validate-044-invalid-plural-forms"
                }
                PoHeaderDiagnostic::InvalidPluralRule(_) => {
                    "diagnostic-validate-044-invalid-plural-rule"
                }
                PoHeaderDiagnostic::PluralHeaderRequired => {
                    "diagnostic-validate-044-plural-header-required"
                }
            },
            Self::Markup(_) => unreachable!("markup presentation is handled above"),
        };
        DiagnosticPresentationId::new_static(value)
    }

    fn kind(&self) -> PoDiagnosticKind {
        match self {
            Self::ExpectedDirective => PoDiagnosticKind::ExpectedDirective,
            Self::ExpectedQuotedString => PoDiagnosticKind::ExpectedQuotedString,
            Self::MissingField(field) => PoDiagnosticKind::MissingField(field),
            Self::DuplicateField(target) => PoDiagnosticKind::DuplicateField(field_name(*target)),
            Self::QuotedContinuationWithoutField => {
                PoDiagnosticKind::QuotedContinuationWithoutField
            }
            Self::UnexpectedTextAfterQuotedString => {
                PoDiagnosticKind::UnexpectedTextAfterQuotedString
            }
            Self::UnterminatedQuotedString => PoDiagnosticKind::UnterminatedQuotedString,
            Self::UnsupportedEscape(value) => PoDiagnosticKind::UnsupportedEscape(value.clone()),
            Self::InvalidStableId(value) => PoDiagnosticKind::InvalidStableId(value.clone()),
            Self::PlaceholderMismatch(value) => {
                PoDiagnosticKind::PlaceholderMismatch(value.clone())
            }
            Self::InvalidPluralArms(cause) => {
                PoDiagnosticKind::InvalidPluralArms(plural_message(cause))
            }
            Self::InvalidHeader(PoHeaderDiagnostic::InvalidPluralRule(reason)) => {
                PoDiagnosticKind::InvalidPluralRule(reason.clone())
            }
            Self::InvalidHeader(cause) => PoDiagnosticKind::InvalidHeader(header_message(cause)),
            Self::InvalidFieldOrder(target) => {
                PoDiagnosticKind::InvalidFieldOrder(format!("unexpected {target:?}"))
            }
            Self::DuplicateKey {
                context,
                source_text,
            } => PoDiagnosticKind::DuplicateKey(format!(
                "context `{context}` and msgid `{source_text}`"
            )),
            Self::Markup(cause) => cause.kind(),
        }
    }

    fn message(&self) -> String {
        match self {
            Self::ExpectedDirective => "expected PO directive".to_owned(),
            Self::ExpectedQuotedString => "expected quoted PO string".to_owned(),
            Self::MissingField(field) => format!("entry is missing {field}"),
            Self::DuplicateField(target) => format!("duplicate PO field {}", field_name(*target)),
            Self::QuotedContinuationWithoutField => {
                "quoted continuation without a PO field".to_owned()
            }
            Self::UnexpectedTextAfterQuotedString => {
                "unexpected text after quoted PO string".to_owned()
            }
            Self::UnterminatedQuotedString => "unterminated quoted PO string".to_owned(),
            Self::UnsupportedEscape(value) => format!("unsupported PO escape {value}"),
            Self::InvalidStableId(value) => format!("invalid stable PO context `{value}`"),
            Self::PlaceholderMismatch(value) => format!("PO placeholder mismatch: {value}"),
            Self::InvalidPluralArms(cause) => plural_message(cause),
            Self::InvalidHeader(cause) => header_message(cause),
            Self::InvalidFieldOrder(target) => {
                format!("invalid PO field order: unexpected {target:?}")
            }
            Self::DuplicateKey {
                context,
                source_text,
            } => {
                format!("duplicate PO catalogue key: context `{context}` and msgid `{source_text}`")
            }
            Self::Markup(cause) => cause.message(),
        }
    }

    fn arguments(&self) -> Vec<(String, DiagnosticArgumentValue)> {
        match self {
            Self::MissingField(field) => vec![("field".to_owned(), string(*field))],
            Self::DuplicateField(target) => vec![("field".to_owned(), string(field_name(*target)))],
            Self::UnsupportedEscape(value) => vec![("escape".to_owned(), string(value))],
            Self::InvalidStableId(value) => vec![("context".to_owned(), string(value))],
            Self::PlaceholderMismatch(value) => vec![("detail".to_owned(), string(value))],
            Self::InvalidFieldOrder(target) => {
                vec![("value".to_owned(), string(format!("unexpected {target:?}")))]
            }
            Self::DuplicateKey {
                context,
                source_text,
            } => vec![
                ("context".to_owned(), string(context)),
                ("source_text".to_owned(), string(source_text)),
            ],
            Self::Markup(cause) => cause.arguments(),
            Self::InvalidPluralArms(cause) => match cause {
                PoPluralDiagnostic::ExpectedArm(expected) => {
                    vec![("expected".to_owned(), integer(*expected))]
                }
                PoPluralDiagnostic::Count { expected, actual } => vec![
                    ("expected".to_owned(), integer(*expected)),
                    ("actual".to_owned(), integer(*actual)),
                ],
                PoPluralDiagnostic::InvalidArm(keyword) => {
                    vec![("keyword".to_owned(), string(keyword))]
                }
                PoPluralDiagnostic::ContiguousArms | PoPluralDiagnostic::RequiresPluralSource => {
                    Vec::new()
                }
            },
            Self::InvalidHeader(cause) => match cause {
                PoHeaderDiagnostic::MissingColon(line) => {
                    vec![("line".to_owned(), string(line))]
                }
                PoHeaderDiagnostic::DuplicateOrEmpty(key) => {
                    vec![("key".to_owned(), string(key))]
                }
                PoHeaderDiagnostic::InvalidPluralRule(reason) => {
                    vec![("detail".to_owned(), string(reason.to_string()))]
                }
                PoHeaderDiagnostic::MultipleHeaders
                | PoHeaderDiagnostic::InvalidPluralForms
                | PoHeaderDiagnostic::PluralHeaderRequired => Vec::new(),
            },
            Self::ExpectedDirective
            | Self::ExpectedQuotedString
            | Self::QuotedContinuationWithoutField
            | Self::UnexpectedTextAfterQuotedString
            | Self::UnterminatedQuotedString => Vec::new(),
        }
    }
}
