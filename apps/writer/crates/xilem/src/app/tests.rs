#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::picker;
use crate::session::Session;
use recite_config::WriterCommand;
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;
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
}
