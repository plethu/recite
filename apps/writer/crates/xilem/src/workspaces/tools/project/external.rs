//! External reconciliation executes against the checked-out canonical session.
use super::{FileError, ProjectSession, Success, WorkControl, text};
use crate::workspaces::external::Inspection;
use recite_writer_session::{
    comparison::ComparisonRow,
    project::{ExternalDecision, ExternalResolution},
};
use std::path::PathBuf;

pub(crate) enum Action {
    Compare,
    Resolve(ExternalDecision),
    ExportMerge,
    ImportMerge(PathBuf),
}
impl Action {
    pub(super) fn perform(
        &self,
        project: &mut ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<Success, FileError> {
        let (files, model) = project.parts_mut();
        control.checkpoint()?;
        match self {
            Self::Compare => {
                files.compare_external(model)?;
                Ok(Success::message(
                    "Disk comparison ready. Your current field draft is retained.",
                ))
            }
            Self::Resolve(decision) => {
                let resolution = files.resolve_external_with_control(model, *decision, control)?;
                Ok(adopted(
                    resolution,
                    match decision {
                        ExternalDecision::KeepWriter => {
                            "Kept Writer's document and draft. The next Save replaces the inspected disk version."
                        }
                        ExternalDecision::UseInspectedDisk => {
                            "Loaded the inspected disk version. The previous session is archived."
                        }
                    },
                ))
            }
            Self::ExportMerge => {
                let artifact = files.export_external_merge(model, control)?;
                Ok(Success {
                    message: "Merge draft exported. Open it in your editor, then import the reviewed merge.",
                    archive: Some(artifact.recovery),
                    warning: None,
                })
            }
            Self::ImportMerge(path) => {
                let resolution = files.import_external_merge(model, path, control)?;
                Ok(adopted(
                    resolution,
                    "Reviewed merge adopted as one Undo step. Save writes it over the inspected disk version.",
                ))
            }
        }
    }
}
fn adopted(resolution: ExternalResolution, message: &'static str) -> Success {
    Success {
        message,
        archive: Some(resolution.archive),
        warning: resolution.recovery_error.map(|error| {
            format!(
                "{message}\nRecovery is incomplete: {error}. Retry Save or keep this window open."
            )
        }),
    }
}
pub(super) fn describe(project: &ProjectSession) -> Option<Inspection> {
    let comparison = project.files().external.as_ref()?;
    Some(Inspection {
        path: comparison.path().to_owned(),
        changes: ComparisonRow::source("", comparison.writer_source(), comparison.disk_source())
            .into_iter()
            .filter(|row| row.before != row.after)
            .flat_map(text::change_pages)
            .collect(),
        pending: comparison
            .pending_draft()
            .map(|draft| text::pages(draft.draft_snapshot().as_str())),
        artifact: comparison.artifact().cloned(),
        archive: comparison.archive().map(PathBuf::from),
    })
}
