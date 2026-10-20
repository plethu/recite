//! File ownership and recovery shared by the native Writer frontends.
//! Language and editing semantics remain in the authoring kernel.

mod context;
mod document;
mod error;
mod files;
mod project;
pub mod recovery;

pub use context::{diagnostic_messages, load as load_context, share_saved_source};
pub use document::DocumentSession;
pub use error::FileError;
pub use files::{export_recovery, read_regular, replace_checked, sync_directory};
pub use project::{ProjectDocument, ProjectSession};

pub mod activation;
pub mod location;
pub mod map;
pub mod preferences;
