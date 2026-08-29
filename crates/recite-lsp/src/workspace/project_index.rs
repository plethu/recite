use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use lsp_types::Uri;

use super::{SnapshotGeneration, WorkspaceConfig};
use crate::documents::OpenDocumentStore;
use crate::paths::{file_path_to_uri, project_relative_path, uri_to_file_path};
use crate::summary::{FileIdentity, FileSummary, SavedFileIdentity};
use recite_config::DiscoveredDocument;

pub(crate) struct LiveProjectSnapshot {
    #[allow(dead_code)]
    generation: SnapshotGeneration,
    #[allow(dead_code)]
    summaries: Vec<FileSummary>,
}

impl LiveProjectSnapshot {
    pub(super) fn rebuild(
        generation: SnapshotGeneration,
        saved: &SavedProjectIndex,
        documents: &OpenDocumentStore,
    ) -> Self {
        let open_saved_paths = documents
            .documents()
            .filter_map(|document| document.summary().saved_path().map(Path::to_owned))
            .collect::<BTreeSet<_>>();
        let open_uris = documents
            .documents()
            .map(|document| document.summary().uri().as_str().to_owned())
            .collect::<BTreeSet<_>>();

        let mut summaries = saved
            .documents
            .iter()
            .filter(|(path, document)| {
                !open_saved_paths.contains(*path)
                    && !open_uris.contains(document.summary.uri().as_str())
            })
            .map(|(_, document)| document.summary.clone())
            .collect::<Vec<_>>();
        summaries.extend(
            documents
                .documents()
                .map(|document| document.summary().clone()),
        );
        summaries.sort_by(summary_sort_key);

        Self {
            generation,
            summaries,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn generation(&self) -> SnapshotGeneration {
        self.generation
    }

    #[allow(dead_code)]
    pub(crate) fn summaries(&self) -> &[FileSummary] {
        &self.summaries
    }
}

fn summary_sort_key(left: &FileSummary, right: &FileSummary) -> std::cmp::Ordering {
    let left_path = left.project_relative_path().unwrap_or(left.uri().as_str());
    let right_path = right
        .project_relative_path()
        .unwrap_or(right.uri().as_str());
    left_path
        .cmp(right_path)
        .then_with(|| left.uri().cmp(right.uri()))
}

pub(super) struct SavedProjectIndex {
    project_root: PathBuf,
    roots: Vec<PathBuf>,
    documents: BTreeMap<PathBuf, SavedDocument>,
    manifest: Option<recite_config::ProjectManifest>,
    diagnostics: Vec<recite_core::Diagnostic>,
    manifest_path: Option<PathBuf>,
    manifest_text: String,
    discovery_start: Option<PathBuf>,
    discovery_failed: bool,
    aliases: BTreeMap<PathBuf, (PathBuf, bool)>,
}

impl SavedProjectIndex {
    pub(super) fn discover(config: &WorkspaceConfig) -> Self {
        let mut index = Self {
            project_root: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().project_root().to_owned())
                .or_else(|| config.discovery_start.clone())
                .or_else(|| config.roots.first().cloned())
                .unwrap_or_default(),
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
            aliases: BTreeMap::new(),
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
            for root in &config.roots {
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
        if self
            .manifest
            .as_ref()
            .is_some_and(|manifest| !manifest.allows_path(&path))
        {
            self.documents.remove(&path);
            return true;
        }

        if let Some(canonical) = canonical_before
            && lexical_path != canonical
        {
            let direct_target = self
                .documents
                .values()
                .any(|document| document.summary.saved_path() == Some(canonical.as_path()));
            self.aliases
                .insert(lexical_path, (canonical, direct_target));
        }
        self.refresh_path(&path) || removed
    }

    fn remove_uri(
        &mut self,
        uri: &Uri,
        lexical_path: &Path,
        canonical_path: Option<&Path>,
    ) -> bool {
        let before = self.documents.len();
        let aliased_path = self.aliases.remove(lexical_path);
        let lexical_is_canonical =
            canonical_path.is_some_and(|canonical| canonical == lexical_path);
        self.documents.retain(|_, document| {
            let matches_uri = document.summary.uri() == uri;
            let matches_canonical =
                lexical_is_canonical && canonical_path == document.summary.saved_path();
            let matches_alias = aliased_path.as_ref().is_some_and(|(aliased, direct)| {
                !direct && Some(aliased.as_path()) == document.summary.saved_path()
            });
            !(matches_uri || matches_canonical || matches_alias)
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
        self.aliases.clear();
        match result {
            Ok(report) => {
                self.project_root = report.manifest().project_root().to_owned();
                self.roots = report
                    .manifest()
                    .roots()
                    .iter()
                    .map(|root| root.path().to_owned())
                    .collect();
                self.discovery_start = Some(report.manifest().project_root().to_owned());
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
                self.manifest = None;
                self.manifest_path = self.manifest_path.clone().or_else(|| {
                    self.discovery_start
                        .as_deref()
                        .map(|path| path.join(recite_config::PROJECT_MANIFEST_FILE))
                });
                self.manifest_text.clear();
                self.diagnostics.clear();
                self.discovery_failed = false;
                for root in self.roots.clone() {
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

    fn refresh_path(&mut self, path: &Path) -> bool {
        if self.discovery_failed {
            return false;
        }
        let Some(_root) = self.root_for_path(path) else {
            return false;
        };
        if self
            .manifest
            .as_ref()
            .is_some_and(|manifest| !manifest.allows_path(path))
        {
            self.documents.remove(path);
            return true;
        }
        if !path.exists() {
            self.documents.remove(path);
            return true;
        }

        let Ok(text) = fs::read_to_string(path) else {
            self.documents.remove(path);
            return true;
        };
        let Some(uri) = file_path_to_uri(path) else {
            self.documents.remove(path);
            return true;
        };
        let Some(project_relative_path) = project_relative_path(&self.project_root, path) else {
            self.documents.remove(path);
            return true;
        };
        let identity = FileIdentity::Saved(SavedFileIdentity {
            uri,
            canonical_path: path.to_owned(),
            project_relative_path,
        });
        let summary = FileSummary::from_text(identity, None, &text);
        self.documents
            .insert(path.to_owned(), SavedDocument { text, summary });
        true
    }

    pub(super) fn root_for_path(&self, path: &Path) -> Option<&Path> {
        self.roots
            .iter()
            .find(|root| path == root.as_path() || path.starts_with(root))
            .map(PathBuf::as_path)
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
        let identity = FileIdentity::Saved(SavedFileIdentity {
            uri,
            canonical_path: document.path().to_owned(),
            project_relative_path: document.key().as_str().to_owned(),
        });
        let summary = FileSummary::from_text(identity, None, document.text());
        self.documents.insert(
            document.path().to_owned(),
            SavedDocument {
                text: document.text().to_owned(),
                summary,
            },
        );
    }
}

#[derive(Clone, Debug)]
pub(super) struct SavedDocument {
    pub(super) text: String,
    pub(super) summary: FileSummary,
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
