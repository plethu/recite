//! Native contexts use the shared launch parser.
pub use recite_writer_session::startup::{EditorCommand, HELP, LaunchOptions, Startup};
use std::path::PathBuf;

/// Project selected before the file-backed shell starts loading.
#[derive(Clone)]
pub struct InitialProject(pub Option<PathBuf>);

/// Session-only command override; it is never written into user preferences.
#[derive(Clone)]
pub struct InitialEditor(pub Option<EditorCommand>);
