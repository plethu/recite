//! Shared, disposable authoring experiment for native GUI comparisons.
//! Parsing, validation, IDs, compilation, and preview use the real Recite crates.

mod document;
mod edits;
mod preview;
mod projection;
mod recovery;
mod workbench;

pub use document::{Document, EditError, ProjectContext};
pub use preview::{Preview, PreviewError, PreviewPage};
pub use projection::{Passage, PassageKind};
pub use recovery::RecoveredDraft;
pub use workbench::{View, Workbench, WorkbenchError};

pub const FIXTURE: &str =
    include_str!("../../../../../fixtures/recite/valid/gui_bakeoff/crossroads.recite");
pub const DOCUMENT_NAME: &str = "crossroads.recite";
