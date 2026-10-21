//! File ownership and recovery shared by the native Writer frontends.
//! Language and editing semantics remain in the authoring kernel.

mod context;
mod error;
mod files;
pub mod project;
pub mod recovery;

pub use context::{diagnostic_messages, load as load_context, share_saved_source};
pub use error::FileError;
pub use files::{export_recovery, read_regular, replace_checked, sync_directory};
pub use project::{DocumentSession, DocumentSessionMut, ProjectDocument, ProjectSession};

pub mod activation;
pub mod location;
pub mod map;
pub mod preferences;

pub mod builds;
pub mod comparison;
pub mod declarations;
pub mod external;
pub mod messages;
pub mod navigation;

pub mod choices;
pub mod localisation;

pub mod preview;

pub mod startup;

pub mod examples;

pub mod palette;

pub mod commands;

pub mod reading;
pub mod typography;
