use super::*;
#[path = "geometry/tests.rs"]
mod geometry;
#[path = "workflow/tests.rs"]
mod workflow;

impl App {
    pub(crate) fn with_writer(writer: Writer) -> Self {
        Self {
            window: writer.window,
            writer: Some(writer),
            show_welcome: false,
            status: String::new(),
            ..Self::empty()
        }
    }
}
