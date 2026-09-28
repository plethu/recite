//! Engine-neutral compiled assets, deterministic session driving, and catalogue lookup.

mod asset;
mod catalog;
mod driver;
mod error;

pub use asset::LoadedDialogue;
pub use catalog::ReciteDialogueCatalog;
pub use driver::{DriverError, SessionDriver, StartRequest};
pub use error::{AdapterError, AdapterErrorKind, AdapterResult};
