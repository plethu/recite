//! Native routing adapts shared scene and beat lookup targets.
use crate::editing::Writer;
pub(super) use recite_writer_session::commands::targets::{Target, rank};
pub(super) trait TargetExt {
    fn open(&self, writer: Writer) -> Result<(), String>;
}
impl TargetExt for Target {
    fn open(&self, writer: Writer) -> Result<(), String> {
        let location = self.location(crate::navigation::snapshot(writer));
        crate::navigation::route(
            writer,
            location,
            None,
            freya::prelude::EventHandler::new(move |result: Result<(), String>| {
                let mut message = writer.message;
                if let Err(error) = result {
                    message.error(error);
                } else if !writer
                    .buffers
                    .model
                    .peek()
                    .as_ref()
                    .is_ok_and(crate::scene::has_no_beats)
                    && let Err(error) = writer.ensure_beat()
                {
                    message.error(error);
                }
            }),
        )
    }
}
pub(super) fn all(mut writer: Writer) -> Vec<Target> {
    let model = writer.buffers.model.read();
    let Ok(model) = model.as_ref() else {
        return Vec::new();
    };
    if writer.files.read().is_some() {
        recite_writer_session::commands::targets::project(model)
    } else {
        match recite_writer_session::commands::targets::examples(model) {
            Ok(targets) => targets,
            Err(error) => {
                writer.message.error(error.to_string());
                Vec::new()
            }
        }
    }
}
