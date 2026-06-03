use crate::i18n::{Messages, MsgId};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum DialogueCatalogMalformedReason {
    ExpectedDirective,
    ExpectedQuotedString,
    MissingContext,
    MissingId,
    MissingTranslation,
    PlaceholderMismatch { detail: String },
    PluralEntriesUnsupported,
    QuotedContinuationWithoutField,
    UnexpectedTextAfterQuotedString,
    UnterminatedQuotedString,
    UnsupportedEscape { escape: String },
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
            Self::PlaceholderMismatch { detail } => messages.format(
                MsgId::CliErrorDialogueCatalogReasonPlaceholderMismatch,
                [("detail", detail.clone())],
            ),
            Self::PluralEntriesUnsupported => {
                messages.text(MsgId::CliErrorDialogueCatalogReasonPluralEntriesUnsupported)
            }
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
            Self::PlaceholderMismatch { detail } => detail.clone(),
            Self::PluralEntriesUnsupported => "plural entries are not supported".to_owned(),
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
