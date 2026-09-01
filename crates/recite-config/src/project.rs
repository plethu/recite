//! Shared project manifest discovery and deterministic source enumeration.
//!
//! This module owns the path and filesystem contract used by authoring clients.
//! It deliberately does not parse dialogue or speak an editor protocol; those
//! concerns remain in the compiler and LSP crates respectively.

mod diagnostics;
mod enumerate;
mod glob;
mod manifest;

pub use diagnostics::{DiscoveryDiagnostic, ProjectDiscoveryError};
pub use enumerate::{
    Coverage, DiscoveredDocument, DiscoveredRoot, allows_unscoped_source_path,
    discover_unscoped_sources,
};
pub use manifest::{
    PROJECT_MANIFEST_FILE, PROJECT_MANIFEST_FORMAT_VERSION, ProjectDiscoveryReport,
    ProjectManifest, discover_project,
};
pub use recite_core::{DocumentKey, DocumentKeyError};
