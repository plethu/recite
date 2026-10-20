//! Freya presentation uses the shared file-session recovery owner.
pub(crate) mod status;
use crate::editing::Writer;

pub(crate) fn retry_project_edit(mut writer: Writer) {
    let result = (|| {
        let model = writer.buffers.model.peek();
        let model = model.as_ref().map_err(|error| error.to_string())?;
        writer
            .files
            .write()
            .as_mut()
            .ok_or("Open a project first.")?
            .finish_project_edit(model)
            .map_err(|error| error.to_string())
    })();
    writer.buffers.sync(writer.dark);
    writer.message.report(
        result,
        "Project recovery completed. Editing is available.".into(),
    );
}
