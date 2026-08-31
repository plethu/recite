use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use super::WorkspaceConfig;
use crate::paths::{file_path_to_uri, uri_to_file_path};
use crate::summary::SavedFileIdentity;
use recite_config::DiscoveredDocument;

#[path = "project_identity.rs"]
mod project_identity;
#[path = "project_manifest.rs"]
mod project_manifest;
#[path = "project_ownership.rs"]
mod project_ownership;

#[derive(Clone)]
pub(super) struct SavedProjectIndex {
    project_root: PathBuf,
    workspace_root: PathBuf,
    fallback_roots: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    pub(super) documents: BTreeMap<PathBuf, SavedDocument>,
    manifest: Option<recite_config::ProjectManifest>,
    diagnostics: Vec<recite_core::Diagnostic>,
    manifest_path: Option<PathBuf>,
    manifest_text: String,
    discovery_start: Option<PathBuf>,
    discovery_failed: bool,
}

impl SavedProjectIndex {
    pub(super) fn discover(config: &WorkspaceConfig) -> Self {
        let mut index = Self {
            project_root: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().project_root().to_owned())
                .unwrap_or_else(|| common_project_root(&config.fallback_roots)),
            workspace_root: common_project_root(&config.fallback_roots),
            fallback_roots: config.fallback_roots.clone(),
            roots: merged_roots(config),
            documents: BTreeMap::new(),
            manifest: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().clone()),
            diagnostics: config.discovery_diagnostics.clone(),
            manifest_path: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().manifest_path().to_owned())
                .or_else(|| config.discovery_manifest_path.clone()),
            manifest_text: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().source().source_text())
                .or_else(|| {
                    config
                        .discovery_manifest_path
                        .as_deref()
                        .and_then(|path| fs::read_to_string(path).ok())
                })
                .unwrap_or_default(),
            discovery_start: config.discovery_start.clone(),
            discovery_failed: config.discovery_failed,
        };
        if let Some(report) = config.discovery.as_ref() {
            index.diagnostics.extend(
                report
                    .diagnostics()
                    .iter()
                    .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic),
            );
            for document in report.documents() {
                index.insert_discovered(document);
            }
            index.insert_fallback_documents(&config.fallback_roots[1..]);
        } else if !index.discovery_failed {
            index.insert_fallback_documents(&config.fallback_roots);
        }
        index
    }

    pub(super) fn document_by_uri(&self, uri: &Uri) -> Option<&SavedDocument> {
        let path = uri_to_file_path(uri)?;
        let path = canonical_or_existing_parent_path(&path)?;
        self.documents.get(&path)
    }

    pub(super) fn diagnostics(&self) -> &[recite_core::Diagnostic] {
        &self.diagnostics
    }

    pub(super) fn manifest_path(&self) -> Option<&Path> {
        self.manifest_path.as_deref()
    }

    pub(super) fn manifest_text(&self) -> &str {
        &self.manifest_text
    }

    pub(super) fn document_uris(&self) -> impl Iterator<Item = &Uri> {
        self.documents
            .values()
            .map(|document| &document.identity.uri)
    }

    fn insert_discovered(&mut self, document: &DiscoveredDocument) {
        let Some(uri) = file_path_to_uri(document.path()) else {
            return;
        };
        let Some(project_relative_path) = self.project_key_for_path(document.path()) else {
            return;
        };
        let identity = SavedFileIdentity {
            uri,
            canonical_path: document.path().to_owned(),
            project_relative_path,
        };
        let mut source_paths = self
            .documents
            .get(document.path())
            .map(|saved| saved.source_paths.clone())
            .unwrap_or_default();
        source_paths.extend(document.source_paths().iter().cloned());
        self.documents.insert(
            document.path().to_owned(),
            SavedDocument {
                text: document.text().to_owned(),
                identity,
                source_paths,
            },
        );
    }

    fn insert_fallback_documents(&mut self, roots: &[PathBuf]) {
        for root in roots {
            let (documents, diagnostics) = recite_config::discover_unscoped_sources(root);
            self.diagnostics.extend(
                diagnostics
                    .iter()
                    .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic),
            );
            for document in documents {
                self.insert_discovered(&document);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SavedDocument {
    pub(super) text: String,
    pub(super) identity: SavedFileIdentity,
    source_paths: BTreeSet<PathBuf>,
}

fn common_project_root(roots: &[PathBuf]) -> PathBuf {
    let Some(first) = roots.first() else {
        return PathBuf::new();
    };
    let mut common = first.clone();
    for root in &roots[1..] {
        while !root.starts_with(&common) {
            if !common.pop() {
                return PathBuf::new();
            }
        }
    }
    common
}

pub(super) fn merged_roots(config: &WorkspaceConfig) -> Vec<PathBuf> {
    let mut roots = config.roots.clone();
    append_unique_paths(&mut roots, &config.fallback_roots);
    roots
}

pub(super) fn append_unique_paths(roots: &mut Vec<PathBuf>, additional: &[PathBuf]) {
    for root in additional {
        if !roots.iter().any(|existing| existing == root) {
            roots.push(root.clone());
        }
    }
}

fn canonical_or_existing_parent_path(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Some(path);
    }
    let parent = path.parent()?;
    let parent = fs::canonicalize(parent).ok()?;
    let file_name = path.file_name()?;
    Some(parent.join(file_name))
}
