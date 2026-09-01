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
pub(super) mod project_diagnostics;
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
    lexical_roots: Vec<PathBuf>,
    fallback_roots: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    pub(super) documents: BTreeMap<PathBuf, SavedDocument>,
    discoveries: Vec<WorkspaceDiscovery>,
    manifest_diagnostics: BTreeMap<PathBuf, ManifestDiagnostics>,
    partition_completeness: BTreeMap<String, bool>,
}

impl SavedProjectIndex {
    pub(super) fn discover(config: &WorkspaceConfig) -> Self {
        Self::from_discoveries(
            config.lexical_roots.clone(),
            config.fallback_roots.clone(),
            config.discoveries.clone(),
        )
    }

    pub(super) fn from_discoveries(
        lexical_roots: Vec<PathBuf>,
        fallback_roots: Vec<PathBuf>,
        discoveries: Vec<WorkspaceDiscovery>,
    ) -> Self {
        let mut index = Self {
            workspace_root: common_project_root(&fallback_roots),
            lexical_roots,
            roots: roots_for_discoveries(&fallback_roots, &discoveries),
            fallback_roots,
            documents: BTreeMap::new(),
            discoveries,
            manifest_diagnostics: BTreeMap::new(),
            partition_completeness: BTreeMap::new(),
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
                } => {
                    index.add_manifest_diagnostics_value(manifest_path, text, diagnostics);
                }
            }
        }
        index.recompute_partition_completeness();
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

    pub(super) fn partition_ids(&self) -> BTreeSet<String> {
        let mut partitions = BTreeSet::new();
        for discovery in &self.discoveries {
            match &discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => {
                    partitions.insert(crate::paths::stable_path_identity(
                        report.manifest().project_root(),
                    ));
                    if report.manifest().project_root() != discovery.root {
                        partitions.insert(crate::paths::stable_path_identity(&discovery.root));
                    }
                }
                WorkspaceDiscoveryState::Manifestless | WorkspaceDiscoveryState::Failed { .. } => {
                    partitions.insert(crate::paths::stable_path_identity(&discovery.root));
                }
            }
        }
        partitions.insert("standalone".to_owned());
        partitions
    }

    pub(super) fn partition_is_complete(&self, partition: &str) -> bool {
        self.partition_completeness
            .get(partition)
            .copied()
            .unwrap_or(false)
    }

    pub(super) fn refresh_discovery_metadata(&mut self) -> bool {
        let discoveries = super::config::discover_workspace_roots(&self.fallback_roots);
        let mut manifest_diagnostics = BTreeMap::new();
        for discovery in &discoveries {
            match &discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => {
                    let diagnostics = report
                        .diagnostics()
                        .iter()
                        .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                        .collect::<Vec<_>>();
                    if !diagnostics.is_empty() {
                        manifest_diagnostics.insert(
                            report.manifest().manifest_path().to_owned(),
                            ManifestDiagnostics {
                                path: report.manifest().manifest_path().to_owned(),
                                text: report.manifest().source().source_text().to_owned(),
                                diagnostics,
                            },
                        );
                    }
                }
                WorkspaceDiscoveryState::Failed {
                    manifest_path,
                    text,
                    diagnostics,
                } => {
                    if !diagnostics.is_empty() {
                        manifest_diagnostics.insert(
                            manifest_path.clone(),
                            ManifestDiagnostics {
                                path: manifest_path.clone(),
                                text: text.clone(),
                                diagnostics: diagnostics.clone(),
                            },
                        );
                    }
                }
                WorkspaceDiscoveryState::Manifestless => {}
            }
        }
        let previous_completeness = self.partition_completeness.clone();
        let previous_diagnostics = self.manifest_diagnostics.clone();
        self.discoveries = discoveries;
        self.manifest_diagnostics = manifest_diagnostics;
        self.recompute_partition_completeness();
        previous_completeness != self.partition_completeness
            || previous_diagnostics != self.manifest_diagnostics
    }

    fn recompute_partition_completeness(&mut self) {
        self.partition_completeness.clear();
        for discovery in &self.discoveries {
            match &discovery.state {
                WorkspaceDiscoveryState::Manifest(report) => {
                    self.partition_completeness
                        .entry(crate::paths::stable_path_identity(
                            report.manifest().project_root(),
                        ))
                        .and_modify(|complete| *complete &= report.is_complete())
                        .or_insert(report.is_complete());
                    if report.manifest().project_root() != discovery.root {
                        let (_, diagnostics) =
                            recite_config::discover_unscoped_sources(&discovery.root);
                        let complete = diagnostics
                            .iter()
                            .all(recite_config::DiscoveryDiagnostic::is_warning);
                        self.partition_completeness
                            .entry(crate::paths::stable_path_identity(&discovery.root))
                            .and_modify(|current| *current &= complete)
                            .or_insert(complete);
                    }
                }
                WorkspaceDiscoveryState::Manifestless => {
                    let (_, diagnostics) =
                        recite_config::discover_unscoped_sources(&discovery.root);
                    let complete = diagnostics
                        .iter()
                        .all(recite_config::DiscoveryDiagnostic::is_warning);
                    self.partition_completeness
                        .entry(crate::paths::stable_path_identity(&discovery.root))
                        .and_modify(|current| *current &= complete)
                        .or_insert(complete);
                }
                WorkspaceDiscoveryState::Failed { .. } => {
                    self.partition_completeness
                        .entry(crate::paths::stable_path_identity(&discovery.root))
                        .and_modify(|complete| *complete = false)
                        .or_insert(false);
                }
            }
        }
        // A standalone partition has no discovery report proving coverage of
        // the surrounding filesystem. Keep it conservative even when one or
        // more explicitly opened documents happen to be available.
        self.partition_completeness
            .entry("standalone".to_owned())
            .or_insert(false);
    }

    pub(super) fn partition_for_path(&self, path: &Path) -> Option<String> {
        self.project_identity_for_path(path)
            .map(|identity| identity.partition)
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

pub(super) fn canonical_or_existing_parent_path(path: &Path) -> Option<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Some(path);
    }
    let parent = path.parent()?;
    let parent = fs::canonicalize(parent).ok()?;
    let file_name = path.file_name()?;
    Some(parent.join(file_name))
}

pub(super) fn canonical_event_path(path: &Path) -> Option<PathBuf> {
    let parent = path.parent()?;
    let parent = fs::canonicalize(parent).ok()?;
    Some(parent.join(path.file_name()?))
}
