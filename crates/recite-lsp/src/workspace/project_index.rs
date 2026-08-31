use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use super::WorkspaceConfig;
use super::config::{WorkspaceDiscovery, WorkspaceDiscoveryState};
use crate::paths::{file_path_to_uri, uri_to_file_path};
use crate::summary::SavedFileIdentity;
use recite_config::DiscoveredDocument;

#[path = "project_diagnostics.rs"]
mod project_diagnostics;
#[path = "project_identity.rs"]
mod project_identity;
#[path = "project_manifest.rs"]
mod project_manifest;
#[path = "project_ownership.rs"]
mod project_ownership;

use project_diagnostics::ManifestDiagnostics;
pub(super) use project_identity::PathScope;

#[derive(Clone)]
pub(super) struct SavedProjectIndex {
    workspace_root: PathBuf,
    fallback_roots: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    pub(super) documents: BTreeMap<PathBuf, SavedDocument>,
    discoveries: Vec<WorkspaceDiscovery>,
    manifest_diagnostics: BTreeMap<PathBuf, ManifestDiagnostics>,
}

impl SavedProjectIndex {
    pub(super) fn discover(config: &WorkspaceConfig) -> Self {
        Self::from_discoveries(config.fallback_roots.clone(), config.discoveries.clone())
    }

    pub(super) fn from_discoveries(
        fallback_roots: Vec<PathBuf>,
        discoveries: Vec<WorkspaceDiscovery>,
    ) -> Self {
        let mut index = Self {
            workspace_root: common_project_root(&fallback_roots),
            roots: roots_for_discoveries(&fallback_roots, &discoveries),
            fallback_roots,
            documents: BTreeMap::new(),
            discoveries,
            manifest_diagnostics: BTreeMap::new(),
        };
        for discovery in index.discoveries.clone() {
            match discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => {
                    index.add_manifest_diagnostics(&report);
                    for document in report.documents() {
                        index.insert_discovered(document);
                    }
                    if report.manifest().project_root() != discovery.root {
                        index.insert_fallback_documents(std::slice::from_ref(&discovery.root));
                    }
                }
                WorkspaceDiscoveryState::Manifestless => {
                    index.insert_fallback_documents(std::slice::from_ref(&discovery.root));
                }
                WorkspaceDiscoveryState::Failed {
                    manifest_path,
                    text,
                    diagnostics,
                } => index.add_manifest_diagnostics_value(manifest_path, text, diagnostics),
            }
        }
        index
    }

    pub(super) fn document_by_uri(&self, uri: &Uri) -> Option<&SavedDocument> {
        let path = uri_to_file_path(uri)?;
        let path = canonical_or_existing_parent_path(&path)?;
        self.documents.get(&path)
    }

    pub(super) fn discoveries(&self) -> &[WorkspaceDiscovery] {
        &self.discoveries
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
            if self.discoveries.iter().any(|discovery| {
                discovery.root == *root
                    && matches!(discovery.state, WorkspaceDiscoveryState::Failed { .. })
            }) {
                continue;
            }
            if self.discoveries.iter().any(|discovery| {
                matches!(&discovery.state, WorkspaceDiscoveryState::Manifest(report)
                    if root == report.manifest().project_root())
            }) {
                continue;
            }
            let (documents, _diagnostics) = recite_config::discover_unscoped_sources(root);
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

pub(super) fn roots_for_discoveries(
    fallback_roots: &[PathBuf],
    discoveries: &[WorkspaceDiscovery],
) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for discovery in discoveries {
        if let WorkspaceDiscoveryState::Manifest(report) = &discovery.state {
            for root in report.manifest().roots() {
                if !roots.iter().any(|existing| existing == root.path()) {
                    roots.push(root.path().to_owned());
                }
            }
        }
    }
    roots
        .into_iter()
        .chain(fallback_roots.iter().cloned())
        .collect()
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
