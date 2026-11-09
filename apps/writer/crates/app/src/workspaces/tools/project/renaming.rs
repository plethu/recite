use super::*;
use recite_writer_session::comparison::ComparisonRow;
impl Action {
    pub fn perform(
        &self,
        project: &mut ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<Success, FileError> {
        match self {
            Self::External(action) => action.perform(project, control),
            Self::ReviewRename { block, name } => {
                let (files, model) = project.parts_mut();
                control.checkpoint()?;
                files.review_rename(model, block, name)?;
                control.checkpoint()?;
                Ok(Success::message(
                    "Rename reviewed. Inspect every change, then Apply project rename.",
                ))
            }
            Self::ApplyRename => {
                let (files, model) = project.parts_mut();
                files.apply_rename_with_control(model, control)?;
                Ok(Success::message(
                    "Project rename staged. Save all to write the project files.",
                ))
            }
            Self::History { redo } => {
                let (files, model) = project.parts_mut();
                if files.rename_history_with_control(model, *redo, control)? {
                    Ok(Success::message(
                        "Grouped project history restored. Save all to write the project files.",
                    ))
                } else {
                    Ok(Success::message(
                        "No grouped project change is available for this document.",
                    ))
                }
            }
            Self::FinishRecovery => {
                let (files, model) = project.parts_mut();
                files.finish_project_edit(model)?;
                Ok(Success::message(
                    "Project recovery completed. Editing is available.",
                ))
            }
        }
    }
}
pub(super) fn describe(project: &ProjectSession) -> Vec<rename::Change> {
    let Some(review) = &project.files().rename_review else {
        return vec![];
    };
    let rows = review
        .plan
        .changes
        .iter()
        .flat_map(|change| {
            ComparisonRow::source(change.document.as_str(), &change.before, &change.after)
        })
        .chain(
            (review.manifest_before != review.manifest_after)
                .then(|| {
                    ComparisonRow::source(
                        "recite.project.toml",
                        &review.manifest_before,
                        &review.manifest_after,
                    )
                })
                .into_iter()
                .flatten(),
        );
    rows.filter(|row| row.before != row.after)
        .flat_map(text::change_pages)
        .collect()
}
