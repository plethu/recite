use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use super::WorkspaceConfig;
use crate::paths::{file_path_to_uri, project_relative_path, uri_to_file_path};
use crate::summary::{FileIdentity, FileSummary, SavedFileIdentity};
use recite_config::DiscoveredDocument;

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

    pub(super) fn refresh_uri(&mut self, uri: &Uri) -> bool {
        // Remove the old canonical entry before resolving the replacement.
        // A URI can be retargeted through a symlink, and retaining the old key
        // would leave stale or duplicate summaries in the project snapshot.
        let Some(path) = uri_to_file_path(uri) else {
            return false;
        };
        let lexical_path = path.clone();
        let canonical_before = fs::canonicalize(&path).ok();
        let removed = self.remove_uri(uri, &lexical_path, canonical_before.as_deref());
        if self.discovery_failed {
            return removed;
        }
        let Some(path) = canonical_or_existing_parent_path(&path) else {
            return removed;
        };
        if !has_recite_extension(&path) {
            return removed;
        }
        self.refresh_path(&path, &lexical_path) || removed
    }

    fn remove_uri(
        &mut self,
        uri: &Uri,
        lexical_path: &Path,
        canonical_path: Option<&Path>,
    ) -> bool {
        let before = self.documents.len();
        self.documents.retain(|_, document| {
            let owns_source = document.source_paths.remove(lexical_path);
            let matches_uri = document.summary.uri() == uri;
            let matches_canonical = canonical_path == document.summary.saved_path()
                && lexical_path == document.summary.saved_path().unwrap_or(lexical_path);
            let remove_document = (owns_source || matches_uri || matches_canonical)
                && document.source_paths.is_empty();
            !remove_document
        });
        before != self.documents.len()
    }

    /// Re-read the manifest and replace the saved project state atomically.
    /// A failed manifest leaves no saved documents; callers may still layer
    /// open editor buffers on top of the diagnostic-only state.
    pub(super) fn refresh_manifest(&mut self) {
        let Some(start) = self.discovery_start.clone() else {
            return;
        };
        self.apply_discovery(recite_config::discover_project(start));
    }

    fn apply_discovery(
        &mut self,
        result: Result<recite_config::ProjectDiscoveryReport, recite_config::ProjectDiscoveryError>,
    ) {
        self.documents.clear();
        match result {
            Ok(report) => {
                self.project_root = report.manifest().project_root().to_owned();
                self.roots = report
                    .manifest()
                    .roots()
                    .iter()
                    .map(|root| root.path().to_owned())
                    .collect();
                self.manifest = Some(report.manifest().clone());
                self.manifest_path = Some(report.manifest().manifest_path().to_owned());
                self.manifest_text = report.manifest().source().source_text();
                self.diagnostics = report
                    .diagnostics()
                    .iter()
                    .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                    .collect();
                self.discovery_failed = false;
                for document in report.documents() {
                    self.insert_discovered(document);
                }
            }
            Err(recite_config::ProjectDiscoveryError::NotFound { .. }) => {
                self.project_root = common_project_root(&self.fallback_roots);
                self.roots = self.fallback_roots.clone();
                self.manifest = None;
                self.manifest_path = self
                    .discovery_start
                    .as_deref()
                    .map(|path| path.join(recite_config::PROJECT_MANIFEST_FILE));
                self.manifest_text.clear();
                self.diagnostics.clear();
                self.discovery_failed = false;
                for root in self.fallback_roots.clone() {
                    let (documents, diagnostics) = recite_config::discover_unscoped_sources(&root);
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
            Err(error) => {
                self.manifest = None;
                self.manifest_path = error.manifest_path().map(Path::to_owned);
                self.manifest_text = self
                    .manifest_path
                    .as_deref()
                    .and_then(|path| fs::read_to_string(path).ok())
                    .unwrap_or_default();
                self.diagnostics = error.diagnostics();
                self.discovery_failed = true;
            }
        }
    }

    fn refresh_path(&mut self, path: &Path, source_path: &Path) -> bool {
        if self.discovery_failed {
            return false;
        }
        if !self.paths_share_source_root(source_path, path) {
            return false;
        }
        let allowed = self.manifest.as_ref().map_or(
            recite_config::allows_unscoped_source_path(&self.project_root, source_path)
                && recite_config::allows_unscoped_source_path(&self.project_root, path),
            |manifest| manifest.allows_path(source_path) && manifest.allows_path(path),
        );
        if !allowed {
            self.remove_unowned_document(path);
            return true;
        }
        if !path.exists() {
            return true;
        }

        let Ok(text) = fs::read_to_string(path) else {
            self.remove_unowned_document(path);
            return true;
        };
        let Some(uri) = file_path_to_uri(path) else {
            self.remove_unowned_document(path);
            return true;
        };
        let Some(project_relative_path) = project_relative_path(&self.project_root, path) else {
            self.remove_unowned_document(path);
            return true;
        };
        let identity = FileIdentity::Saved(SavedFileIdentity {
            uri,
            canonical_path: path.to_owned(),
            project_relative_path,
        });
        let summary = FileSummary::from_text(identity, None, &text);
        let source_paths = self
            .documents
            .get(path)
            .map(|document| {
                let mut paths = document.source_paths.clone();
                paths.insert(source_path.to_owned());
                paths
            })
            .unwrap_or_else(|| BTreeSet::from([source_path.to_owned()]));
        self.documents.insert(
            path.to_owned(),
            SavedDocument {
                text,
                summary,
                source_paths,
            },
        );
        true
    }

    fn remove_unowned_document(&mut self, path: &Path) {
        if self
            .documents
            .get(path)
            .is_some_and(|document| document.source_paths.is_empty())
        {
            self.documents.remove(path);
        }
    }

    pub(super) fn root_for_path(&self, path: &Path) -> Option<&Path> {
        self.roots
            .iter()
            .find(|root| path == root.as_path() || path.starts_with(root))
            .map(PathBuf::as_path)
    }

    fn paths_share_source_root(&self, source_path: &Path, canonical_path: &Path) -> bool {
        self.roots
            .iter()
            .any(|root| source_path.starts_with(root) && canonical_path.starts_with(root))
    }

    pub(super) fn document_by_uri(&self, uri: &Uri) -> Option<&SavedDocument> {
        let path = uri_to_file_path(uri)?;
        let path = canonical_or_existing_parent_path(&path)?;
        self.documents.get(&path)
    }

    pub(super) fn documents(&self) -> impl Iterator<Item = &SavedDocument> {
        self.documents.values()
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
            .map(|document| document.summary.uri())
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
        let identity = FileIdentity::Saved(SavedFileIdentity {
            uri,
            canonical_path: document.path().to_owned(),
            project_relative_path,
        });
        let summary = FileSummary::from_text(identity, None, document.text());
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
                summary,
                source_paths,
            },
        );
    }
}

#[derive(Clone, Debug)]
pub(super) struct SavedDocument {
    pub(super) text: String,
    pub(super) summary: FileSummary,
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

fn has_recite_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "recite")
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
