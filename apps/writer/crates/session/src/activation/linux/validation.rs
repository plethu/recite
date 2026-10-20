//! Validate forwarded file intent before publishing anything into the live window.
use super::Request;
use std::path::{Path, PathBuf};
pub(super) fn canonical_intent(request: &Request) -> Result<Option<PathBuf>, String> {
    let linked = request
        .route
        .as_deref()
        .map(crate::location::project_from_route)
        .transpose()?
        .flatten();
    let canonical = |path: &Path| {
        recite_config::discover_project(path)
            .map(|report| report.manifest().project_root().to_owned())
            .map_err(|error| format!("Could not open linked project {}: {error}", path.display()))
    };
    match (request.project.as_deref(), linked.as_deref()) {
        (Some(project), Some(linked)) => {
            let project = canonical(project)?;
            if project != canonical(linked)? {
                return Err("The link identifies another project".into());
            }
            Ok(Some(project))
        }
        (Some(project), None) => canonical(project).map(Some),
        (None, Some(_)) => Err("A project link needs a project intent".into()),
        (None, None) => Ok(None),
    }
}

pub(super) fn validate_options(request: &Request, project: Option<&Path>) -> Result<(), String> {
    if let Some(document) = &request.document {
        crate::startup::validate_document(
            project.ok_or("A document activation requires a project")?,
            document,
            request.route.as_deref(),
        )?;
    }
    if request
        .editor
        .as_ref()
        .is_some_and(|command| command.program.is_empty())
    {
        return Err("The external editor program cannot be empty".into());
    }
    Ok(())
}
