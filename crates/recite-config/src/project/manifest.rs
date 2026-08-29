use std::path::{Path, PathBuf};

use recite_core::ProjectManifestSource;

use super::diagnostics::{DiscoveryDiagnostic, ProjectDiscoveryError};
use super::enumerate::{Coverage, DiscoveredDocument, DiscoveredRoot};
use super::glob::GlobPattern;

#[path = "manifest_discovery.rs"]
mod discovery;
#[path = "manifest_search.rs"]
mod search;

pub const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";
pub const PROJECT_MANIFEST_FORMAT_VERSION: u32 = 1;

/// Shared project manifest plus its canonical filesystem interpretation.
#[derive(Clone, Debug)]
pub struct ProjectManifest {
    project_root: PathBuf,
    manifest_path: PathBuf,
    source: ProjectManifestSource,
    roots: Vec<DiscoveredRoot>,
    excludes: Vec<String>,
    exclude_patterns: Vec<GlobPattern>,
}

impl ProjectManifest {
    #[must_use]
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    #[must_use]
    pub fn manifest_path(&self) -> &Path {
        &self.manifest_path
    }

    #[must_use]
    pub fn source(&self) -> &ProjectManifestSource {
        &self.source
    }

    #[must_use]
    pub fn roots(&self) -> &[DiscoveredRoot] {
        &self.roots
    }

    #[must_use]
    pub fn excludes(&self) -> &[String] {
        &self.excludes
    }

    /// Whether a canonical path is an eligible source path under this
    /// manifest's built-in and configured exclusion rules.
    #[must_use]
    pub fn allows_path(&self, path: &Path) -> bool {
        self.allows_event_path(path)
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".recite"))
    }

    /// Whether a watcher event is inside a configured root and not excluded.
    /// Unlike [`Self::allows_path`], this also accepts directories and missing
    /// paths so delete/rename events can wake a rebuild.
    #[must_use]
    pub fn allows_event_path(&self, path: &Path) -> bool {
        let Ok(relative) = path.strip_prefix(&self.project_root) else {
            return false;
        };
        let relative = relative.to_string_lossy().replace('\\', "/");
        !relative.is_empty()
            && !relative.split('/').any(|name| name.starts_with('.'))
            && !relative.split('/').any(is_builtin_excluded)
            && !self
                .exclude_patterns
                .iter()
                .any(|pattern| pattern.matches(&relative))
            && self
                .roots
                .iter()
                .any(|root| path.starts_with(root.path()) || root.path().starts_with(path))
    }
}

/// Deterministic project source index. Valid documents survive independent
/// coverage failures so an LSP can remain useful while publishing diagnostics.
#[derive(Clone, Debug)]
pub struct ProjectDiscoveryReport {
    manifest: ProjectManifest,
    documents: Vec<DiscoveredDocument>,
    diagnostics: Vec<DiscoveryDiagnostic>,
    coverage: Coverage,
}

impl ProjectDiscoveryReport {
    #[must_use]
    pub fn manifest(&self) -> &ProjectManifest {
        &self.manifest
    }

    #[must_use]
    pub fn documents(&self) -> &[DiscoveredDocument] {
        &self.documents
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[DiscoveryDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub const fn coverage(&self) -> Coverage {
        self.coverage
    }

    #[must_use]
    pub const fn is_complete(&self) -> bool {
        self.coverage.is_complete()
    }
}

/// Find and index the first `recite.project.toml` at or above an invocation
/// path. A malformed first manifest is an error; discovery never falls through
/// to a parent project after finding one.
pub fn discover_project(
    path: impl AsRef<Path>,
) -> Result<ProjectDiscoveryReport, ProjectDiscoveryError> {
    discovery::discover_project(path)
}

fn is_builtin_excluded(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "build" | "dist" | "out" | "generated" | "vendor" | "node_modules"
        )
}
