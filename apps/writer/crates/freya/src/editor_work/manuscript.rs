//! The native adapter harvests widgets around the shared durability sequence.
use super::{
    document,
    worker::{Output, Work, Worker},
};
use crate::{editing::Writer, project::FileError};
use recite_writer_session::project::save::sequence::{Progress, SaveSequence};

pub(super) async fn save(
    mut writer: Writer,
    scope: super::SaveScope,
    request: Option<recite_writer_model::EditRequest>,
    worker: &Worker,
) -> Result<(), String> {
    if let Some(request) = request
        && let Err(error) = document::accept(writer, request, worker).await
    {
        writer.buffers.harvest();
        let mut files = writer.files.write();
        let mut model = writer.buffers.model.write();
        let project = files
            .as_mut()
            .ok_or(FileError::Selection)
            .map_err(|e| e.to_string())?;
        project.set_saving(false);
        let model = model.as_mut().map_err(|e| e.to_string())?;
        let _ = project.queue_checkpoint(model);
        return project
            .finish_save(model, Err(error))
            .map_err(|e| e.to_string());
    }
    writer.buffers.harvest();
    crate::declarations::harvest(writer);
    let mut sequence = {
        let mut files = writer.files.write();
        let model = writer.buffers.model.peek();
        SaveSequence::accepted(
            files
                .as_mut()
                .ok_or(FileError::Selection)
                .map_err(|e| e.to_string())?,
            model.as_ref().map_err(|e| e.to_string())?,
            scope,
        )
    };
    loop {
        writer.buffers.harvest();
        crate::declarations::harvest(writer);
        let progress = {
            let mut files = writer.files.write();
            let mut model = writer.buffers.model.write();
            sequence.poll(
                files
                    .as_mut()
                    .ok_or(FileError::Selection)
                    .map_err(|e| e.to_string())?,
                model.as_mut().map_err(|e| e.to_string())?,
            )
        };
        let work = match progress {
            Progress::Work(work) => work,
            Progress::Finished(result) => return result.map_err(|e| e.to_string()),
            Progress::Waiting => {
                sequence.failed(FileError::SavePending);
                continue;
            }
        };
        let output = worker.execute(Work::Save(work)).await;
        // Typing may have continued during any I/O, including declaration output.
        writer.buffers.harvest();
        crate::declarations::harvest(writer);
        let mut files = writer.files.write();
        let mut model = writer.buffers.model.write();
        let project = files
            .as_mut()
            .ok_or(FileError::Selection)
            .map_err(|e| e.to_string())?;
        let model = model.as_mut().map_err(|e| e.to_string())?;
        match output {
            Ok(Output::Save(receipt)) => sequence.accept(project, model, receipt),
            Ok(_) => sequence.failed(FileError::Selection),
            Err(error) => sequence.failed(error),
        }
    }
}
