use super::*;
#[path = "geometry/tests.rs"]
mod geometry;
#[path = "workflow/tests.rs"]
mod workflow;

impl App {
    pub(crate) fn window_id(&self) -> WindowId {
        self.window
    }
    pub(crate) fn with_writer(writer: Writer) -> Self {
        Self {
            writer: Some(writer),
            show_welcome: false,
            status: String::new(),
            ..Self::empty()
        }
    }
}
