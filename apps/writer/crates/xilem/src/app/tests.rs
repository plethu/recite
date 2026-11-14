#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::picker;
use crate::session::Session;
use crate::tests::gallery::stage::Look;
use recite_config::WriterCommand;
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;
use xilem::masonry::widgets::Passthrough;
#[path = "geometry/tests.rs"]
mod geometry;
#[path = "workflow/tests.rs"]
mod workflow;

impl App {
    pub(crate) fn empty() -> Self {
        Self::wrap(Shell::empty())
    }
    pub(crate) fn window_id(&self) -> WindowId {
        self.window
    }
    pub(crate) fn with_writer(writer: Writer) -> Self {
        Self::wrap(Shell::with_writer(writer))
    }
    /// The look the native window would use for the current preferences.
    pub(crate) fn gallery_look(&mut self, initial: bool) -> Look {
        self.theme.refresh(&self.shell.preferences.config.writer);
        Look {
            properties: self.theme.properties.clone(),
            canvas: self.theme.palette.canvas,
            initial: initial.then(|| self.theme.initial_properties()),
        }
    }
}

/// The whole window content, as the native window mounts it.
pub(crate) fn gallery_page(
    app: &mut App,
) -> impl xilem::WidgetView<App, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), view::app(app))
}
