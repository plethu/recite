//! Ordered manuscript, manifest and declaration durability steps.
use super::{
    document,
    worker::{Output, Work, Worker},
};
use crate::{
    editing::Writer,
    project::{
        FileError,
        save::ordered::{Drafts, Step},
    },
};

pub(super) async fn save(
    mut writer: Writer,
    scope: super::SaveScope,
    request: Option<recite_writer_model::EditRequest>,
    worker: &Worker,
) -> Result<(), String> {
    let result = async {
        if let Some(request) = request {
            document::accept(writer, request, worker).await?;
        }
        let steps = {
            let files = writer.files.peek();
            let model = writer.buffers.model.peek();
            files
                .as_ref()
                .ok_or(FileError::Selection)?
                .save_plan_accepted(model.as_ref().map_err(|_| FileError::Selection)?, scope)?
        };
        run(writer, steps, worker).await
    }
    .await;
    writer.buffers.harvest();
    let mut files = writer.files.write();
    let mut model = writer.buffers.model.write();
    let project = files
        .as_mut()
        .ok_or("The saved project is no longer open.")?;
    project.set_saving(false);
    project
        .finish_save(model.as_mut().map_err(|e| e.to_string())?, result)
        .map_err(|e| e.to_string())
}
async fn run(mut writer: Writer, steps: Vec<Step>, worker: &Worker) -> Result<(), FileError> {
    for step in steps {
        match step {
            Step::Manifest => {
                let request = writer
                    .files
                    .peek()
                    .as_ref()
                    .ok_or(FileError::Selection)?
                    .manifest_save();
                if let Some(request) = request {
                    let Output::Manifest(outcome) = worker.execute(Work::Manifest(request)).await?
                    else {
                        return Err(FileError::Selection);
                    };
                    writer
                        .files
                        .write()
                        .as_mut()
                        .ok_or(FileError::Selection)?
                        .accept_manifest_save(outcome)?;
                }
            }
            Step::Declarations { force } => save_declarations(writer, worker, force).await?,
            mut step => {
                let request = writer
                    .files
                    .peek()
                    .as_ref()
                    .ok_or(FileError::Selection)?
                    .retained_edit(&step)?;
                if let Some((path, request)) = request {
                    let Output::Edit(result) = worker.execute(Work::Edit(request)).await? else {
                        return Err(FileError::Selection);
                    };
                    let mut prepared = result?;
                    let accepted = writer
                        .files
                        .write()
                        .as_mut()
                        .ok_or(FileError::Selection)
                        .and_then(|files| files.accept_retained_edit(&path, &mut prepared));
                    worker.execute(Work::RetireEdit(prepared)).await?;
                    accepted?;
                    step = Step::Retained {
                        path,
                        drafts: Drafts::Retain,
                    };
                }
                let write = writer
                    .files
                    .write()
                    .as_mut()
                    .ok_or(FileError::Selection)?
                    .prepare_write(step)?;
                let Output::Written(mut write, result) = worker.execute(Work::Write(write)).await?
                else {
                    return Err(FileError::Selection);
                };
                let path = write.path.clone();
                let publication = result.and_then(|()| {
                    writer.buffers.harvest();
                    let model = writer.buffers.model.peek();
                    let model = model.as_ref().map_err(|_| FileError::Selection)?;
                    writer
                        .files
                        .write()
                        .as_mut()
                        .ok_or(FileError::Selection)?
                        .publish_write(model, &mut write)
                });
                // Disk and publication errors must also retire their large owners.
                worker.execute(Work::RetireWrite(write)).await?;
                let ticket = publication?;
                let Output::Recovered(result) = worker.execute(Work::Recovery(ticket)).await?
                else {
                    return Err(FileError::Selection);
                };
                let park = writer
                    .files
                    .write()
                    .as_mut()
                    .ok_or(FileError::Selection)?
                    .finish_saved_tab(&path);
                result.and(park)?;
            }
        }
    }
    Ok(())
}
async fn save_declarations(
    mut writer: Writer,
    worker: &Worker,
    force: bool,
) -> Result<(), FileError> {
    crate::declarations::harvest(writer);
    let request = {
        let files = writer.files.peek();
        let project = files.as_ref().ok_or(FileError::Selection)?;
        let Some(session) = project.declarations.as_ref().filter(|s| force || s.dirty()) else {
            return Ok(());
        };
        session.prepare_save()?
    };
    let Output::DeclarationSource(request, result) = worker
        .execute(Work::DeclarationSource(Box::new(request)))
        .await?
    else {
        return Err(FileError::Selection);
    };
    result?;
    crate::declarations::harvest(writer);
    let ticket = writer
        .files
        .write()
        .as_mut()
        .and_then(|p| p.declarations.as_mut())
        .ok_or(FileError::Selection)?
        .accept_saved_source(&request)?;
    let Output::Recovered(result) = worker.execute(Work::DeclarationRecovery(ticket)).await? else {
        return Err(FileError::Selection);
    };
    result?;
    let Output::GeneratedDeclarations(request, result) =
        worker.execute(Work::PublishDeclarations(request)).await?
    else {
        return Err(FileError::Selection);
    };
    result?;
    // Refresh may replace declaration ownership; preserve edits made during output I/O.
    crate::declarations::harvest(writer);
    let mut files = writer.files.write();
    let project = files.as_mut().ok_or(FileError::Selection)?;
    project
        .declarations
        .as_mut()
        .ok_or(FileError::Selection)?
        .accept_saved_output(*request);
    // Schema refresh uses the live accepted overlay after durable publication.
    project.set_saving(false);
    let result = match writer.buffers.model.write().as_mut() {
        Ok(model) => project.refresh(model),
        Err(_) => Err(FileError::Selection),
    };
    project.set_saving(true);
    result
}
