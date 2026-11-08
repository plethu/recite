//! External reconciliation executes against the checked-out canonical session.
use super::{Effect, FileError, ProjectSession, Success, WorkControl};
use crate::workspaces::external::{Inspection, Resolution, Resolved};
use recite_writer_session::project::{ExternalDecision, ExternalResolution};
use std::path::PathBuf;

pub enum Action {
    Compare,
    Resolve(ExternalDecision),
    ExportMerge,
    ImportMerge(PathBuf),
}
impl Action {
    pub fn perform(
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
                    Resolved {
                        path: files.current.clone(),
                        resolution: match decision {
                            ExternalDecision::KeepWriter => Resolution::KeepWriter,
                            ExternalDecision::UseInspectedDisk => Resolution::UseInspectedDisk,
                        },
                    },
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
                    effect: Effect::None,
                })
            }
            Self::ImportMerge(path) => {
                let resolution = files.import_external_merge(model, path, control)?;
                Ok(adopted(
                    resolution,
                    Resolved {
                        path: files.current.clone(),
                        resolution: Resolution::Merged,
                    },
                    "Reviewed merge adopted as one Undo step. Save writes it over the inspected disk version.",
                ))
            }
        }
    }
}
fn adopted(resolution: ExternalResolution, effect: Resolved, message: &'static str) -> Success {
    Success {
        message,
        archive: Some(resolution.archive),
        warning: resolution.recovery_error.map(|error| {
            format!(
                "{message}\nRecovery is incomplete: {error}. Retry Save or keep this window open."
            )
        }),
        effect: Effect::ExternalResolved(effect),
    }
}
pub fn describe(project: &ProjectSession) -> Option<Inspection> {
    let comparison = project.files().external.as_ref()?;
    Some(Inspection::prepare(comparison))
}
