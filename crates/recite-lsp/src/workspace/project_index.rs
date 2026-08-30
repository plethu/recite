use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use super::WorkspaceConfig;
use crate::paths::{file_path_to_uri, project_relative_path, uri_to_file_path};
use crate::summary::SavedFileIdentity;
use recite_config::DiscoveredDocument;

#[path = "project_manifest.rs"]
mod project_manifest;
#[path = "project_ownership.rs"]
mod project_ownership;

pub(super) struct SavedProjectIndex {
    project_root: PathBuf,
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
            fallback_roots: config.fallback_roots.clone(),
            roots: config.roots.clone(),
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
        } else if !index.discovery_failed {
            for root in &config.fallback_roots {
                let (documents, diagnostics) = recite_config::discover_unscoped_sources(root);
                index.diagnostics.extend(
                    diagnostics
                        .iter()
                        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic),
                );
                for document in documents {
                    index.insert_discovered(&document);
                }
            }
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

    pub(super) fn project_key_for_path(&self, path: &Path) -> Option<String> {
        project_relative_path(&self.project_root, path)
    }

    fn insert_discovered(&mut self, document: &DiscoveredDocument) {
        let Some(uri) = file_path_to_uri(document.path()) else {
            return;
        };
        let project_relative_path = if self.manifest.is_some() {
            document.key().as_str().to_owned()
        } else {
            project_relative_path(&self.project_root, document.path())
                .unwrap_or_else(|| document.key().as_str().to_owned())
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

fn canonical_or_existing_parent_path(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Some(path);
    }
    let parent = path.parent()?;
    let parent = fs::canonicalize(parent).ok()?;
    let file_name = path.file_name()?;
    Some(parent.join(file_name))
}
