//! Shared project manifest discovery and deterministic source enumeration.
//!
//! This module owns the path and filesystem contract used by authoring clients.
//! It deliberately does not parse dialogue or speak an editor protocol; those
//! concerns remain in the compiler and LSP crates respectively.

mod diagnostics;
mod enumerate;
mod glob;
mod manifest;
mod schema;

pub use schema::{LoadedProjectSchema, ProjectSchemaError};

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

mod settings;
pub use settings::{ProjectSettings, ProjectSettingsError};

fn project_relative_key(project_root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(project_root).ok()?;
    let mut key = String::new();
    for component in relative.components() {
        let component = component.as_os_str().to_str()?;
        if !key.is_empty() {
            key.push('/');
        }
        key.push_str(component);
    }
    Some(key)
}

#[cfg(test)]
mod tests;
