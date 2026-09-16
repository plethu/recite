//! Poll only this small status component; ordinary editing never waits for disk.
use crate::{design::tokens as t, project::ProjectFiles};
use freya::prelude::*;
#[derive(Clone, PartialEq)]
pub(crate) struct RecoveryStatus {
    pub files: State<Option<ProjectFiles>>,
}
impl Component for RecoveryStatus {
    fn render(&self) -> impl IntoElement {
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(250));
        if tick.elapsed() {
            tick.reset();
        }
        let error = self
            .files
            .peek()
            .as_ref()
            .and_then(ProjectFiles::recovery_error);
        rect().maybe_child(error.map(|error| {
            label()
                .text(format!(
                    "Recovery failed: {error}. Retry Save or keep editing."
                ))
                .font_size(t::TEXT_SMALL)
        }))
    }
}
