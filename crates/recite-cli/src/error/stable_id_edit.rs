use crate::i18n::{Messages, MsgId};

/// A refused `id rename` or `id reassign` request.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum StableIdEditError {
    Reserved { id: String, character: char },
    Malformed { id: String },
    InUse { id: String },
    InCatalog { id: String, catalog: String },
    Unknown { id: String },
    Ambiguous { id: String },
    Target { target: String },
    Unchanged { id: String },
    CatalogMalformed { catalog: String },
}

impl std::fmt::Display for StableIdEditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Reserved { id, character } => write!(
                formatter,
                "stable ID `{id}` contains `{character}`, which is reserved and never appears in an ID"
            ),
            Self::Malformed { id } => write!(formatter, "stable ID `{id}` is malformed"),
            Self::InUse { id } => write!(
                formatter,
                "stable ID `@{id}` is already used by a line or choice"
            ),
            Self::InCatalog { id, catalog } => write!(
                formatter,
                "stable ID `@{id}` still keys entries in catalogue {catalog}"
            ),
            Self::Unknown { id } => {
                write!(formatter, "no line or choice carries stable ID `@{id}`")
            }
            Self::Ambiguous { id } => write!(
                formatter,
                "stable ID `@{id}` is carried by more than one header; select one as PATH:LINE"
            ),
            Self::Target { target } => write!(formatter, "no stable ID header at {target}"),
            Self::Unchanged { id } => write!(formatter, "stable ID `@{id}` already has that name"),
            Self::CatalogMalformed { catalog } => {
                write!(formatter, "catalogue {catalog} is not valid PO")
            }
        }
    }
}

impl StableIdEditError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::Reserved { id, character } => messages.format(
                MsgId::CliErrorStableIdReserved,
                [("id", id.clone()), ("character", character.to_string())],
            ),
            Self::Malformed { id } => {
                messages.format(MsgId::CliErrorStableIdMalformed, [("id", id.clone())])
            }
            Self::InUse { id } => {
                messages.format(MsgId::CliErrorStableIdInUse, [("id", id.clone())])
            }
            Self::InCatalog { id, catalog } => messages.format(
                MsgId::CliErrorStableIdInCatalog,
                [("id", id.clone()), ("catalog", catalog.clone())],
            ),
            Self::Unknown { id } => {
                messages.format(MsgId::CliErrorStableIdUnknown, [("id", id.clone())])
            }
            Self::Ambiguous { id } => {
                messages.format(MsgId::CliErrorStableIdAmbiguous, [("id", id.clone())])
            }
            Self::Target { target } => {
                messages.format(MsgId::CliErrorStableIdTarget, [("target", target.clone())])
            }
            Self::Unchanged { id } => {
                messages.format(MsgId::CliErrorStableIdUnchanged, [("id", id.clone())])
            }
            Self::CatalogMalformed { catalog } => messages.format(
                MsgId::CliErrorStableIdCatalogMalformed,
                [("catalog", catalog.clone())],
            ),
        }
    }
}
