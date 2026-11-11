//! Refusals from `catalog` and `recording`: dialogue catalogues, sidecars and handoff files.

use std::fmt;

use crate::i18n::{Messages, MsgId};

#[derive(Debug)]
pub(crate) enum LocalisationError {
    CatalogMalformed { catalog: String, detail: String },
    CatalogRefresh { catalog: String, detail: String },
    CatalogChanged { catalog: String },
    RecordingLocale { locale: String },
    RecordingSidecar { sidecar: String, detail: String },
    RecordingHandoff { file: String, detail: String },
}

impl fmt::Display for LocalisationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CatalogMalformed { catalog, detail } => {
                write!(formatter, "catalogue {catalog} is not valid PO: {detail}")
            }
            Self::CatalogRefresh { catalog, detail } => {
                write!(formatter, "cannot refresh catalogue {catalog}: {detail}")
            }
            Self::CatalogChanged { catalog } => write!(
                formatter,
                "catalogue {catalog} changed during the refresh; run the refresh again"
            ),
            Self::RecordingLocale { locale } => write!(
                formatter,
                "no catalogue declares Language `{locale}`; add one to [discovery].catalogs"
            ),
            Self::RecordingSidecar { sidecar, detail } => {
                write!(
                    formatter,
                    "recording sidecar {sidecar} is invalid: {detail}"
                )
            }
            Self::RecordingHandoff { file, detail } => {
                write!(formatter, "handoff {file} is invalid: {detail}")
            }
        }
    }
}

impl LocalisationError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::CatalogMalformed { catalog, detail } => messages.format(
                MsgId::CliErrorCatalogMalformed,
                [("catalog", catalog.clone()), ("detail", detail.clone())],
            ),
            Self::CatalogRefresh { catalog, detail } => messages.format(
                MsgId::CliErrorCatalogRefresh,
                [("catalog", catalog.clone()), ("detail", detail.clone())],
            ),
            Self::CatalogChanged { catalog } => messages.format(
                MsgId::CliErrorCatalogChanged,
                [("catalog", catalog.clone())],
            ),
            Self::RecordingLocale { locale } => {
                messages.format(MsgId::CliErrorRecordingLocale, [("locale", locale.clone())])
            }
            Self::RecordingSidecar { sidecar, detail } => messages.format(
                MsgId::CliErrorRecordingSidecar,
                [("sidecar", sidecar.clone()), ("detail", detail.clone())],
            ),
            Self::RecordingHandoff { file, detail } => messages.format(
                MsgId::CliErrorRecordingHandoff,
                [("file", file.clone()), ("detail", detail.clone())],
            ),
        }
    }
}

impl From<LocalisationError> for super::CliError {
    fn from(error: LocalisationError) -> Self {
        Self::Localisation(error)
    }
}
