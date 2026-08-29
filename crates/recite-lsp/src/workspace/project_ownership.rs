use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use lsp_types::Uri;

use super::{SavedDocument, SavedProjectIndex, canonical_or_existing_parent_path};
use crate::paths::{file_path_to_uri, project_relative_path, uri_to_file_path};
use crate::summary::{FileIdentity, FileSummary, SavedFileIdentity};

impl SavedProjectIndex {
    pub(crate) fn refresh_uri(&mut self, uri: &Uri) -> bool {
        let Some(lexical_path) = uri_to_file_path(uri) else {
            return false;
        };
        let canonical_before = fs::canonicalize(&lexical_path).ok();
        let direct_target = canonical_before.as_deref() == Some(lexical_path.as_path())
            || self.documents.get(&lexical_path).is_some_and(|document| {
                document.summary.saved_path() == Some(lexical_path.as_path())
            });
        let removed = if direct_target {
            self.remove_canonical_document(canonical_before.as_deref().unwrap_or(&lexical_path))
        } else {
            self.remove_uri(uri, &lexical_path, canonical_before.as_deref())
        };
        if self.discovery_failed {
            return removed;
        }
        let Some(path) = canonical_or_existing_parent_path(&lexical_path) else {
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

    fn refresh_path(&mut self, path: &Path, source_path: &Path) -> bool {
        if self.discovery_failed || !self.paths_share_source_root(source_path, path) {
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

    fn remove_canonical_document(&mut self, path: &Path) -> bool {
        self.documents.remove(path).is_some()
    }

    fn paths_share_source_root(&self, source_path: &Path, canonical_path: &Path) -> bool {
        self.roots
            .iter()
            .any(|root| source_path.starts_with(root) && canonical_path.starts_with(root))
    }
}

fn has_recite_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "recite")
}
