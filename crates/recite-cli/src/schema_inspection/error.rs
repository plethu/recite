use std::path::PathBuf;

use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub(crate) enum SchemaInspectionError {
    Json(serde_json::Error),
    UnsupportedFormat { path: PathBuf, format: String },
    Malformed { path: PathBuf, format: &'static str },
    InvalidSummary { reason: String },
}

impl SchemaInspectionError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::Json(error) => messages.format(
                MsgId::CliErrorSchemaInspectionJson,
                [("error", error.to_string())],
            ),
            Self::UnsupportedFormat { path, format } => messages.format(
                MsgId::CliErrorSchemaInspectionUnsupportedFormat,
                [("path", display_path(path)), ("format", format.clone())],
            ),
            Self::Malformed { path, format } => messages.format(
                MsgId::CliErrorSchemaInspectionMalformed,
                [
                    ("path", display_path(path)),
                    ("format", (*format).to_owned()),
                ],
            ),
            Self::InvalidSummary { reason } => messages.format(
                MsgId::CliErrorSchemaInspectionInvalidSummary,
                [("reason", reason.clone())],
            ),
        }
    }
}

impl std::fmt::Display for SchemaInspectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Json(error) => {
                write!(
                    formatter,
                    "failed to encode schema inspection JSON: {error}"
                )
            }
            Self::UnsupportedFormat { path, format } => write!(
                formatter,
                "unsupported schema inspection format `{format}` for {}",
                display_path(path)
            ),
            Self::Malformed { path, format } => write!(
                formatter,
                "malformed {format} schema input {}",
                display_path(path)
            ),
            Self::InvalidSummary { reason } => {
                write!(formatter, "invalid schema inspection summary: {reason}")
            }
        }
    }
}

impl std::error::Error for SchemaInspectionError {}
