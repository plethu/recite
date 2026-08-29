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
    roots: Vec<PathBuf>,
    documents: BTreeMap<PathBuf, SavedDocument>,
    manifest: Option<recite_config::ProjectManifest>,
    diagnostics: Vec<recite_core::Diagnostic>,
    manifest_path: Option<PathBuf>,
    manifest_text: String,
}

impl SavedProjectIndex {
    pub(super) fn discover(config: &WorkspaceConfig) -> Self {
        let mut index = Self {
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
                .or_else(|| {
                    config
                        .roots
                        .first()
                        .map(|root| root.join("recite.project.toml"))
                }),
            manifest_text: config
                .discovery
                .as_ref()
                .map(|report| report.manifest().source().source_text())
                .unwrap_or_default(),
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
        } else {
            let mut paths = Vec::new();
            for root in &config.roots {
                collect_recite_files(root, &mut paths);
            }
            paths.sort();
            paths.dedup();
            for path in paths {
                index.refresh_path(&path);
            }
        }
        index
    }

    pub(super) fn refresh_uri(&mut self, uri: &Uri) -> bool {
        let Some(path) = uri_to_file_path(uri) else {
            return false;
        };
        let Some(path) = canonical_or_existing_parent_path(&path) else {
            return false;
        };
        if !has_recite_extension(&path) {
            return false;
        }
        if self
            .manifest
            .as_ref()
            .is_some_and(|manifest| !manifest.allows_path(&path))
        {
            self.documents.remove(&path);
            return true;
        }

        self.refresh_path(&path)
    }

    fn refresh_path(&mut self, path: &Path) -> bool {
        let Some(root) = self.root_for_path(path) else {
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
        let Some(project_relative_path) = project_relative_path(root, path) else {
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

fn collect_recite_files(root: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            if should_skip_directory(&path) {
                continue;
            }
            collect_recite_files(&path, paths);
        } else if file_type.is_file()
            && has_recite_extension(&path)
            && let Ok(path) = fs::canonicalize(path)
        {
            paths.push(path);
        }
    }
}

fn should_skip_directory(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    name.starts_with('.')
        || matches!(
            name,
            "target" | "build" | "dist" | "out" | "generated" | "vendor" | "node_modules"
        )
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
