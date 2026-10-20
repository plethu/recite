//! Poll only this small status component; ordinary editing never waits for disk.
use crate::{design::tokens as t, project::ProjectFiles};
use freya::prelude::*;
#[derive(Clone, PartialEq)]
pub(crate) struct RecoveryStatus {
    pub files: State<Option<ProjectFiles>>,
    pub retry: EventHandler<()>,
}
impl Component for RecoveryStatus {
    fn render(&self) -> impl IntoElement {
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(250));
        if tick.elapsed() {
            tick.reset();
        }
        let pending = self
            .files
            .read()
            .as_ref()
            .is_some_and(ProjectFiles::project_edit_recovery_pending);
        let retry = self.retry.clone();
        let error = self
            .files
            .peek()
            .as_ref()
            .and_then(ProjectFiles::recovery_error);
        rect()
            .maybe_child(pending.then(|| pending_recovery(retry)))
            .maybe_child(error.filter(|_| !pending).map(|error| {
                label()
                    .text(format!(
                        "Recovery failed: {error}. Retry Save or keep editing."
                    ))
                    .font_size(t::small())
            }))
    }
}

fn pending_recovery(retry: EventHandler<()>) -> impl IntoElement {
    rect()
        .spacing(t::SPACE_SM)
        .child(label()
            .text("Grouped project recovery is incomplete. Editing is paused; retry recovery or close and keep recovery.")
            .font_size(t::small()))
        .child(crate::design::Button::new()
            .named("Retry project edit recovery")
            .on_press(move |_| retry.call(()))
            .child("Retry project edit recovery"))
}
