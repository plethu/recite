//! Forwarded native launches publish only into an available, matching session.
use super::*;
use std::sync::atomic;
impl Shell {
    pub fn accept_activation(&mut self, request: recite_writer_session::activation::IncomingRoute) {
        let reply = request.reply.clone();
        let result = (|| {
            if request.cancelled.load(atomic::Ordering::Acquire) {
                return Err("Writer activation was cancelled".into());
            }
            if let Some(writer) = self.writer.as_mut()
                && request.project.as_ref().is_none_or(|path| {
                    writer
                        .session
                        .project()
                        .is_some_and(|project| project.root() == path)
                })
            {
                if writer.phase != Phase::Writing || writer.work.busy() || writer.navigation.busy()
                {
                    return Err("Finish the current editor action before opening a link.".into());
                }
                if let Some(route) = &request.route {
                    let location = route.parse().map_err(str::to_owned)?;
                    writer.navigate_activation(location, request)?;
                } else {
                    if let Some(editor) = request.editor {
                        writer.editor = Some(editor);
                    }
                    let _ = request.reply.send(Ok(()));
                }
                self.show_welcome = false;
                self.settings_open = false;
                return Ok(());
            }
            let options = LaunchOptions {
                intent: Startup::Writer {
                    project: request.project,
                    route: request.route,
                },
                document: request.document,
                editor: request.editor,
            };
            self.start_open(options, Some(request.reply), Some(request.cancelled))
        })();
        if let Err(error) = result {
            self.status = error.clone();
            let _ = reply.send(Err(error));
        }
    }
}
