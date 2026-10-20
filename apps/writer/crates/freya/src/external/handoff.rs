//! The OS owns preferred-application registration. Paths are passed as argv.
use crate::{
    editing::Writer,
    messages::{MsgId, text},
};
use recite_writer_session::external::Handoff;
pub(crate) fn open_editor(mut writer: Writer) {
    if writer
        .files
        .peek()
        .as_ref()
        .is_some_and(|p| p.handoff.is_some())
    {
        return;
    }
    crate::editor_work::start(
        writer,
        crate::editor_work::SaveScope::Current,
        freya::prelude::EventHandler::new(move |result| {
            if let Err(error) = result {
                writer.message.error(error);
                return;
            }
            launch(writer);
        }),
    );
}
fn launch(mut writer: Writer) {
    let mut files = writer.files.write();
    let Some(project) = files.as_mut() else {
        return;
    };
    match Handoff::configured(
        project.current.clone(),
        writer.editor_command.peek().as_ref(),
    ) {
        Ok(job) => {
            project.handoff = Some(job);
            writer.message.info(text(MsgId::WriterExternalOpening));
        }
        Err(e) => writer.message.error(e),
    }
}
