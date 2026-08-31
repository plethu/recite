use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use lsp_types::Uri;

use super::{
    SavedDocument, SavedProjectIndex, canonical_event_path, canonical_or_existing_parent_path,
};
use crate::paths::{file_path_to_uri, uri_to_file_path};
use crate::summary::SavedFileIdentity;

impl SavedProjectIndex {
    pub(crate) fn refresh_uri(&mut self, uri: &Uri) -> bool {
        let Some(lexical_path) = uri_to_file_path(uri) else {
            return false;
        };
        // Reconcile the lexical source first. The path may have changed from
        // an alias to a regular file (or a directory), so classifying it from
        // the new filesystem state would otherwise strand the old canonical
        // document and its source ownership.
        let had_canonical_document = self.documents.contains_key(&lexical_path);
        let mut removed = self.remove_uri(uri, &lexical_path);
        // A direct target event must invalidate the canonical document even
        // when canonicalization now fails (for example after deletion).
        if had_canonical_document {
            removed = self.remove_canonical_document(&lexical_path) || removed;
        }
        // Preserve lexical exclusion semantics before resolving aliases to a
        // canonical target.  Otherwise `.hidden/link.recite` could become a
        // visible `link.recite` after canonicalization.
        if self.is_lexically_excluded(&lexical_path) {
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

    fn remove_uri(&mut self, uri: &Uri, lexical_path: &Path) -> bool {
        let mut changed = false;
        self.documents.retain(|_, document| {
            let owns_source = document.source_paths.remove(lexical_path);
            let matches_uri = document.identity.uri == *uri;
            let remove_document = (owns_source || matches_uri) && document.source_paths.is_empty();
            changed |= owns_source || remove_document;
            !remove_document
        });
        changed
    }

    fn refresh_path(&mut self, path: &Path, source_path: &Path) -> bool {
        let source_root_path =
            canonical_event_path(source_path).unwrap_or_else(|| source_path.to_owned());
        if !self.paths_share_source_root(&source_root_path, path) {
            return false;
        }
        let Some(project_relative_path) = self.project_key_for_path(path) else {
            self.remove_unowned_document(path);
            return true;
        };
        if self.project_key_for_path(&source_root_path).is_none() {
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
        let identity = SavedFileIdentity {
            uri,
            canonical_path: path.to_owned(),
            project_relative_path,
        };
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
                identity,
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

    fn is_lexically_excluded(&self, path: &Path) -> bool {
        self.lexical_roots.iter().chain(&self.roots).any(|root| {
            path.starts_with(root) && !recite_config::allows_unscoped_source_path(root, path)
        })
    }
}

fn has_recite_extension(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "recite")
}
