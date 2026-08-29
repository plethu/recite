use crate::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, MarkupUnbalancedKind,
    PoDiagnosticKind,
};

use super::builder::string;

const NEW_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE048");
const UNBALANCED_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE023");
const MISSING_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE049");
const ATTRIBUTE_CHANGE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE047");

pub(crate) enum MarkupDiagnostic {
    UnknownTag(String),
    UnbalancedTag {
        tag: String,
        kind: MarkupUnbalancedKind,
    },
    MissingTag(String),
    AttributeChange {
        tag: String,
        expected: String,
        actual: String,
    },
}

impl MarkupDiagnostic {
    pub(super) fn code(&self) -> DiagnosticCode {
        match self {
            Self::UnknownTag(_) => NEW_TAG,
            Self::UnbalancedTag { .. } => UNBALANCED_TAG,
            Self::MissingTag(_) => MISSING_TAG,
            Self::AttributeChange { .. } => ATTRIBUTE_CHANGE,
        }
    }

    pub(super) fn presentation_id(&self) -> DiagnosticPresentationId {
        let value = match self {
            Self::UnknownTag(_) => "diagnostic-validate-048",
            Self::UnbalancedTag { kind, .. } => match kind {
                MarkupUnbalancedKind::MissingClosingBracket => "diagnostic-validate-023-bracket",
                MarkupUnbalancedKind::Standalone => "diagnostic-validate-023-standalone",
                MarkupUnbalancedKind::NoOpening => "diagnostic-validate-023-no-opening",
                MarkupUnbalancedKind::Mismatch { .. } => "diagnostic-validate-023-mismatch",
            },
            Self::MissingTag(_) => "diagnostic-validate-049",
            Self::AttributeChange { .. } => "diagnostic-validate-047",
        };
        DiagnosticPresentationId::new_static(value)
    }

    pub(super) fn kind(&self) -> PoDiagnosticKind {
        match self {
            Self::UnknownTag(tag) => PoDiagnosticKind::MarkupUnknownTag(tag.clone()),
            Self::UnbalancedTag { tag, kind } => {
                PoDiagnosticKind::MarkupUnbalancedTag(unbalanced_message(tag, kind))
            }
            Self::MissingTag(tag) => PoDiagnosticKind::MarkupMissingTag(tag.clone()),
            Self::AttributeChange {
                tag,
                expected,
                actual,
            } => PoDiagnosticKind::MarkupAttributeChange {
                tag: tag.clone(),
                expected: expected.clone(),
                actual: actual.clone(),
            },
        }
    }

    pub(super) fn message(&self) -> String {
        match self {
            Self::UnknownTag(tag) => {
                format!("translation introduces inline markup tag `{tag}` not present in msgid")
            }
            Self::UnbalancedTag { tag, kind } => unbalanced_message(tag, kind),
            Self::MissingTag(tag) => {
                format!("translation is missing required inline markup tag `{tag}`")
            }
            Self::AttributeChange {
                tag,
                expected,
                actual,
            } => format!(
                "translation changes attributes for inline markup tag `{tag}`: expected `{expected}`, got `{actual}`"
            ),
        }
    }

    pub(super) fn arguments(&self) -> Vec<(String, DiagnosticArgumentValue)> {
        match self {
            Self::UnknownTag(tag) | Self::MissingTag(tag) => {
                vec![("tag".to_owned(), string(tag))]
            }
            Self::UnbalancedTag { tag, kind } => match kind {
                MarkupUnbalancedKind::MissingClosingBracket => Vec::new(),
                MarkupUnbalancedKind::Standalone | MarkupUnbalancedKind::NoOpening => {
                    vec![("tag".to_owned(), string(tag))]
                }
                MarkupUnbalancedKind::Mismatch { expected } => vec![
                    ("tag".to_owned(), string(tag)),
                    ("expected_tag".to_owned(), string(expected)),
                ],
            },
            Self::AttributeChange {
                tag,
                expected,
                actual,
            } => vec![
                ("tag".to_owned(), string(tag)),
                ("expected".to_owned(), string(expected)),
                ("actual".to_owned(), string(actual)),
            ],
        }
    }
}

fn unbalanced_message(tag: &str, kind: &MarkupUnbalancedKind) -> String {
    match kind {
        MarkupUnbalancedKind::MissingClosingBracket => {
            "unbalanced inline markup tag `[`: missing closing bracket".to_owned()
        }
        MarkupUnbalancedKind::Standalone => format!(
            "unbalanced inline markup tag `{tag}`: standalone tag does not use a closing tag"
        ),
        MarkupUnbalancedKind::NoOpening => {
            format!("unbalanced inline markup tag `{tag}`: closing tag has no matching opening tag")
        }
        MarkupUnbalancedKind::Mismatch { expected } => format!(
            "unbalanced inline markup tag `{tag}`: expected closing tag for `{expected}` first"
        ),
    }
}
