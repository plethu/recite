//! Project access keeps form fields mounted and submission controls in view.
pub use super::Shell;
use std::path::PathBuf;

pub mod feedback;
pub use feedback::{Problem, ProjectAction};

/// A queued submission cannot acquire newer folder, name or mode authority.
#[derive(PartialEq)]
pub struct FormScope {
    creating: bool,
    folder: String,
    name: String,
    native_path: Option<PathBuf>,
    pub(super) ready: bool,
}
impl FormScope {
    pub fn capture(app: &Shell) -> Self {
        Self {
            creating: app.creating,
            folder: app.path.clone(),
            name: app.name.clone(),
            native_path: app.native_path.clone(),
            ready: app.project_form_ready(),
        }
    }
}
