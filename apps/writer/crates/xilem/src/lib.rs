//! The native Xilem frontend for the Recite Writer.
//!
//! Authoring state and workflows live in `recite_writer_app`; this crate renders them, adapts
//! native input and accessibility, and drives their asynchronous work.
// The window's composed view type nests deeper than the default limit lets the compiler prove.
#![recursion_limit = "256"]
mod app;
mod attention;
mod editor_work;
mod external_editor;
mod keyed;
mod localisation;
mod map;
mod native_host;
mod navigation;
mod notices;
mod presentation;
mod prose;
mod reading;
mod reading_context;
mod rules;
mod session;
mod specimen;
#[cfg(test)]
mod tests;
mod trial;
mod view;
mod workspaces;
pub(crate) use recite_writer_app::*;
use recite_writer_session::startup::Startup;
pub use recite_writer_session::startup::{HELP, LaunchOptions};

/// Frontend measurements that depend on the rendered layout rather than authoring state.
pub(crate) trait WriterLayout {
    fn script_width(&self) -> f64;
    /// The width a tool or Localise page sets its content in, on its sheet or full-bleed paper.
    fn page_width(&self) -> f64;
}
impl WriterLayout for Writer {
    fn script_width(&self) -> f64 {
        view::workspace::Layout::resolve(self).script
    }
    fn page_width(&self) -> f64 {
        view::workspace::sheet::page_width(self)
    }
}

pub fn run_launch(options: LaunchOptions) -> Result<(), String> {
    if options.intent == Startup::DesignSystem {
        return specimen::run();
    }
    let Some(app) = app::App::new(options)? else {
        return Ok(());
    };
    app::run(app).map_err(|error| error.to_string())
}
