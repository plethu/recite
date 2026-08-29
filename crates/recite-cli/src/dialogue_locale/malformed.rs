use crate::i18n::{Messages, MsgId};
use recite_core::DiagnosticPresentation;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DialogueCatalogMalformedReason {
    ExpectedDirective,
    ExpectedQuotedString,
    MissingContext,
    MissingId,
    MissingTranslation,
    InvalidHeader {
        detail: String,
    },
    InvalidPluralRule {
        detail: String,
    },
    InvalidStableId {
        value: String,
    },
    DuplicateField {
        field: String,
    },
    DuplicateEntry {
        key: String,
    },
    InvalidFieldOrder {
        detail: String,
    },
    PlaceholderMismatch {
        detail: String,
    },
    Markup {
        presentation: DiagnosticPresentation,
        compatibility_message: String,
    },
    QuotedContinuationWithoutField,
    UnexpectedTextAfterQuotedString,
    UnterminatedQuotedString,
    UnsupportedEscape {
        escape: String,
    },
}

impl DialogueCatalogMalformedReason {
    pub(crate) fn user_message(&self, messages: &Messages) -> String {
        match self {
            Self::ExpectedDirective => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonExpectedDirective)
            }
            Self::ExpectedQuotedString => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonExpectedQuotedString)
            }
            Self::MissingContext => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonMissingContext)
            }
            Self::MissingId => messages.text(MsgId::CliErrorDialogueCatalogReasonMissingId),
            Self::MissingTranslation => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonMissingTranslation)
            }
            Self::InvalidHeader { detail } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonInvalidHeader,
                [("detail", detail.clone())],
            ),
            Self::InvalidPluralRule { detail } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonInvalidPluralRule,
                [("detail", detail.clone())],
            ),
            Self::InvalidStableId { value } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonInvalidStableId,
                [("value", value.clone())],
            ),
            Self::DuplicateField { field } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonDuplicateField,
                [("field", field.clone())],
            ),
            Self::DuplicateEntry { key } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonDuplicateEntry,
                [("key", key.clone())],
            ),
            Self::InvalidFieldOrder { detail } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonInvalidFieldOrder,
                [("detail", detail.clone())],
            ),
            Self::PlaceholderMismatch { detail } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonPlaceholderMismatch,
                [("detail", detail.clone())],
            ),
            Self::Markup { presentation, .. } => messages.format_presentation(presentation),
            Self::QuotedContinuationWithoutField => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonQuotedContinuationWithoutField)
            }
            Self::UnexpectedTextAfterQuotedString => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonUnexpectedTextAfterQuotedString)
            }
            Self::UnterminatedQuotedString => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonUnterminatedQuotedString)
            }
            Self::UnsupportedEscape { escape } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonUnsupportedEscape,
                [("escape", escape.clone())],
            ),
        }
    }

    pub(crate) fn fallback_message(&self) -> String {
        match self {
            Self::ExpectedDirective => "expected msgctxt, msgid, or msgstr".to_owned(),
            Self::ExpectedQuotedString => "expected quoted gettext string".to_owned(),
            Self::MissingContext => "entry is missing msgctxt".to_owned(),
            Self::MissingId => "entry is missing msgid".to_owned(),
            Self::MissingTranslation => "entry is missing msgstr".to_owned(),
            Self::InvalidHeader { detail } => format!("invalid PO header: {detail}"),
            Self::InvalidPluralRule { detail } => {
                format!("invalid PO Plural-Forms rule: {detail}")
            }
            Self::InvalidStableId { value } => format!("invalid stable PO context `{value}`"),
            Self::DuplicateField { field } => format!("duplicate PO field {field}"),
            Self::DuplicateEntry { key } => format!("duplicate PO catalogue entry `{key}`"),
            Self::InvalidFieldOrder { detail } => detail.clone(),
            Self::PlaceholderMismatch { detail } => detail.clone(),
            Self::Markup {
                compatibility_message,
                ..
            } => compatibility_message.clone(),
            Self::QuotedContinuationWithoutField => {
                "quoted continuation without msgctxt, msgid, or msgstr".to_owned()
            }
            Self::UnexpectedTextAfterQuotedString => {
                "unexpected text after quoted string".to_owned()
            }
            Self::UnterminatedQuotedString => "unterminated quoted string".to_owned(),
            Self::UnsupportedEscape { escape } => format!("unsupported escape {escape}"),
        }
    }
}
