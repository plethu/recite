use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use recite_core::DocumentKey;

use super::diagnostics::DiscoveryDiagnostic;
use super::glob::GlobPattern;

#[path = "enumerate_traversal.rs"]
mod traversal;

/// Whether every configured source root and source file was covered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coverage {
    Complete,
    Partial,
}

impl Coverage {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// Canonical source root in manifest declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredRoot {
    pub(super) index: usize,
    pub(super) relative: String,
    pub(super) path: PathBuf,
}

impl DiscoveredRoot {
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    #[must_use]
    pub fn relative(&self) -> &str {
        &self.relative
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// A valid, UTF-8 `.recite` source discovered under a project root.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiscoveredDocument {
    key: DocumentKey,
    path: PathBuf,
    source_paths: Vec<PathBuf>,
    root_index: usize,
    text: String,
}

impl DiscoveredDocument {
    #[must_use]
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    #[must_use]
    pub fn source_paths(&self) -> &[PathBuf] {
        &self.source_paths
    }

    #[must_use]
    pub const fn root_index(&self) -> usize {
        self.root_index
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

pub(super) fn enumerate_root(
    project_root: &Path,
    root: &DiscoveredRoot,
    excludes: &[GlobPattern],
    documents: &mut Vec<DiscoveredDocument>,
    diagnostics: &mut Vec<DiscoveryDiagnostic>,
    seen: &mut BTreeMap<PathBuf, usize>,
) {
    traversal::collect_root(project_root, root, excludes, documents, diagnostics, seen);
}

/// Enumerate a source-only workspace without inventing a project manifest.
/// This is retained for editor compatibility when no manifest exists; callers
/// must not use it after a manifest was found and failed to load.
pub fn discover_unscoped_sources(
    project_root: &Path,
) -> (Vec<DiscoveredDocument>, Vec<DiscoveryDiagnostic>) {
    let canonical_root = match std::fs::canonicalize(project_root) {
        Ok(path) => path,
        Err(error) => {
            return (
                Vec::new(),
                vec![DiscoveryDiagnostic::ReadDirectory {
                    path: project_root.to_owned(),
                    message: error.to_string(),
                }],
            );
        }
    };
    let root = DiscoveredRoot {
        index: 0,
        relative: ".".to_owned(),
        path: canonical_root.clone(),
    };
    let mut documents = Vec::new();
    let mut diagnostics = Vec::new();
    let mut seen = BTreeMap::new();
    enumerate_root(
        &canonical_root,
        &root,
        &[],
        &mut documents,
        &mut diagnostics,
        &mut seen,
    );
    documents.sort_by(|left, right| left.key().cmp(right.key()));
    (documents, diagnostics)
}

/// Whether a canonical path is eligible for the built-in source-only walker.
/// Manifest-backed callers additionally apply their configured roots and
/// custom excludes.
#[must_use]
pub fn allows_unscoped_source_path(project_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(project_root) else {
        return false;
    };
    let relative = relative.to_string_lossy().replace('\\', "/");
    !relative.is_empty()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".recite"))
        && !is_excluded_relative(&relative, &[])
}

fn is_builtin_excluded(name: &str) -> bool {
    name.starts_with('.')
        || matches!(
            name,
            "target" | "build" | "dist" | "out" | "generated" | "vendor" | "node_modules"
        )
}

fn is_excluded_relative(relative: &str, excludes: &[GlobPattern]) -> bool {
    relative.split('/').any(is_builtin_excluded)
        || excludes.iter().any(|glob| glob.matches(relative))
}
