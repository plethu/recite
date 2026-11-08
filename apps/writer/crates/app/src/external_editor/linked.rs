//! A source link owns its reply until the existing launcher has actually started a process.
use super::*;
use crate::navigation::Origin;
use recite_compiler::authoring::CancellationToken;
use recite_writer_session::activation::IncomingRoute;
use std::sync::{Mutex, atomic, mpsc};

pub struct Reply {
    sender: Option<mpsc::Sender<Result<(), String>>>,
    cancelled: Option<Arc<atomic::AtomicBool>>,
}
pub struct Control {
    cancel: CancellationToken,
    reply: Mutex<Option<Reply>>,
}
pub struct Link {
    request: Arc<Request>,
    origin: Origin,
}
impl Control {
    fn new(cancel: CancellationToken, incoming: Option<IncomingRoute>) -> Self {
        Self {
            cancel,
            reply: Mutex::new(Some(Reply {
                sender: incoming.as_ref().map(|incoming| incoming.reply.clone()),
                cancelled: incoming.map(|incoming| incoming.cancelled),
            })),
        }
    }
    pub fn check(&self) -> Result<(), String> {
        self.cancel
            .checkpoint()
            .map_err(|error| error.to_string())?;
        let reply = self.reply.lock().unwrap_or_else(|error| error.into_inner());
        if reply.as_ref().is_some_and(|reply| {
            reply
                .cancelled
                .as_ref()
                .is_some_and(|cancelled| cancelled.load(atomic::Ordering::Acquire))
        }) {
            return Err("Editor launch cancelled; your work stays open.".into());
        }
        Ok(())
    }
    pub fn complete(&self, result: Result<(), String>) {
        let reply = self
            .reply
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .take();
        if let Some(sender) = reply.and_then(|reply| reply.sender) {
            let _ = sender.send(result);
        }
    }
    fn attach(
        &self,
        sender: Option<mpsc::Sender<Result<(), String>>>,
        cancelled: Option<Arc<atomic::AtomicBool>>,
    ) {
        let mut reply = self.reply.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(reply) = reply.as_mut() {
            reply.sender = sender;
            reply.cancelled = cancelled;
        }
    }
}
impl Drop for Control {
    fn drop(&mut self) {
        self.complete(Err(
            "Editor launch stopped before it could start. Your work stays open.".into(),
        ));
    }
}
impl Link {
    pub fn check(&self, writer: &Writer) -> Result<(), String> {
        self.request.check_link()?;
        if !self.origin.is_current(writer) {
            return Err("The source or draft changed while opening the link. Your current work stays open; retry the link.".into());
        }
        Ok(())
    }
    pub fn finish(self, writer: &mut Writer, result: Result<(), String>) {
        match result {
            Ok(()) => writer.launch_editor(self.request),
            Err(error) => {
                self.request.finish_link(Err(error.clone()));
                writer.status = error;
            }
        }
    }
}
impl Writer {
    pub fn queue_linked_editor(&mut self, cancel: CancellationToken) -> Result<(), String> {
        if self.external.busy() || self.source_repair_pending() {
            return Err("Finish the current editor action or repair the recovered source before opening this link.".into());
        }
        let project = self
            .session
            .project()
            .ok_or("Open a project before opening its source in your editor.")?;
        let incoming = self.navigation.take_incoming();
        let editor = incoming
            .as_ref()
            .and_then(|incoming| incoming.editor.clone())
            .or_else(|| self.editor.clone());
        let mut request = Request::source(project.current().path().to_owned(), editor);
        request.link = Some(Arc::new(Control::new(cancel, incoming)));
        self.external.waiting = Some(Link {
            request: Arc::new(request),
            origin: Origin::capture(self),
        });
        self.status = "Preparing source for your editor…".into();
        Ok(())
    }
    pub fn attach_linked_editor_reply(
        &mut self,
        reply: Option<mpsc::Sender<Result<(), String>>>,
        cancelled: Option<Arc<atomic::AtomicBool>>,
    ) -> bool {
        let Some(link) = &self.external.waiting else {
            return false;
        };
        if let Some(control) = &link.request.link {
            control.attach(reply, cancelled);
        }
        true
    }
    pub fn resume_linked_editor(&mut self) {
        if self.work.sender.is_none()
            || self.external.sender.is_none()
            || self.navigation.runtime.is_none()
        {
            return;
        }
        let Some(link) = self.external.waiting.take() else {
            return;
        };
        if let Err(error) = link.check(self) {
            link.finish(self, Err(error));
            return;
        }
        self.save_for_linked_editor(link);
    }
}

#[cfg(test)]
mod tests;
