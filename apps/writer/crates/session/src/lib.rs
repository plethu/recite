//! File ownership and recovery shared by the native Writer frontends.
//! Language and editing semantics remain in the authoring kernel.

mod basic_project;
mod context;
mod document;
mod error;
mod files;
pub mod project;
pub mod recovery;

pub use basic_project::{ProjectDocument, ProjectSession};
pub use context::{diagnostic_messages, load as load_context, share_saved_source};
pub use document::DocumentSession;
pub use error::FileError;
pub use files::{export_recovery, read_regular, replace_checked, sync_directory};

pub mod activation;
pub mod location;
pub mod map;
pub mod preferences;

pub mod builds;
pub mod comparison;
pub mod declarations;
pub mod external;
pub mod messages;

pub mod choices;
pub mod localisation;

pub mod preview;
