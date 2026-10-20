//! Standalone schema source and explicit producer workflows shared by native frontends.
pub mod entries;
pub mod producer;
pub mod recovery;
pub mod session;
pub use session::{DeclarationRevision, DeclarationSave, Session};
