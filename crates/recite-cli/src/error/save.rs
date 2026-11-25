//! Refusals from `save impact` and `save inspect`.

use std::fmt;

use crate::i18n::{Messages, MsgId};

#[derive(Debug)]
pub(crate) enum SaveError {
    /// The project could not be read at the baseline revision.
    Baseline { reference: String, detail: String },
    /// `--since` compares project builds, so the candidate must be a project.
    SinceNeedsProject { candidate: String },
    /// The candidate builds no asset with the baseline's asset ID.
    MissingAsset { asset: String },
    /// The project build failed for a reason other than diagnostics.
    Build { detail: String },
    /// The save file is not a Recite session snapshot this version can read.
    Snapshot { path: String, detail: String },
}

impl fmt::Display for SaveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Baseline { reference, detail } => {
                write!(
                    formatter,
                    "cannot build the project at `{reference}`: {detail}"
                )
            }
            Self::SinceNeedsProject { candidate } => write!(
                formatter,
                "--since compares project builds; {candidate} is not a project directory"
            ),
            Self::MissingAsset { asset } => write!(
                formatter,
                "the candidate builds no asset `{asset}`; compare assets with the same asset ID"
            ),
            Self::Build { detail } => write!(formatter, "the project build failed: {detail}"),
            Self::Snapshot { path, detail } => {
                write!(formatter, "{path} is not a readable Recite save: {detail}")
            }
        }
    }
}

impl SaveError {
    pub(crate) fn to_user_message(&self, messages: &Messages) -> String {
        match self {
            Self::Baseline { reference, detail } => messages.format(
                MsgId::CliErrorSaveBaseline,
                [("reference", reference.clone()), ("detail", detail.clone())],
            ),
            Self::SinceNeedsProject { candidate } => messages.format(
                MsgId::CliErrorSaveSinceNeedsProject,
                [("candidate", candidate.clone())],
            ),
            Self::MissingAsset { asset } => {
                messages.format(MsgId::CliErrorSaveMissingAsset, [("asset", asset.clone())])
            }
            Self::Build { detail } => {
                messages.format(MsgId::CliErrorSaveBuild, [("detail", detail.clone())])
            }
            Self::Snapshot { path, detail } => messages.format(
                MsgId::CliErrorSaveSnapshot,
                [("path", path.clone()), ("detail", detail.clone())],
            ),
        }
    }
}

impl From<SaveError> for super::CliError {
    fn from(error: SaveError) -> Self {
        Self::Save(error)
    }
}
