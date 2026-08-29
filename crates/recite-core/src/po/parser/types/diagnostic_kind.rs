#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PoDiagnosticKind {
    ExpectedDirective,
    ExpectedQuotedString,
    MissingField(&'static str),
    DuplicateField(String),
    QuotedContinuationWithoutField,
    UnexpectedTextAfterQuotedString,
    UnterminatedQuotedString,
    UnsupportedEscape(String),
    InvalidStableId(String),
    PlaceholderMismatch(String),
    InvalidPluralArms(String),
    InvalidPluralRule(crate::PluralRuleError),
    InvalidHeader(String),
    InvalidFieldOrder(String),
    DuplicateKey(String),
    MarkupUnknownTag(String),
    MarkupUnbalancedTag(String),
    MarkupMissingTag(String),
    MarkupAttributeChange {
        tag: String,
        expected: String,
        actual: String,
    },
}
