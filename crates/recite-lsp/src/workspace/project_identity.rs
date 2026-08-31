use std::path::{Path, PathBuf};

use super::SavedProjectIndex;
use crate::paths::{project_relative_path, stable_path_identity};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathScope {
    Project(String),
    Excluded,
    Standalone,
}

impl SavedProjectIndex {
    pub(crate) fn project_key_for_path(&self, path: &Path) -> Option<String> {
        match self.path_scope(path) {
            PathScope::Project(key) => Some(key),
            PathScope::Excluded | PathScope::Standalone => None,
        }
    }

    pub(crate) fn path_scope(&self, path: &Path) -> PathScope {
        let manifest_excluded = self.manifest.as_ref().is_some_and(|manifest| {
            path.starts_with(manifest.project_root()) && !manifest.allows_path(path)
        });
        if let Some(manifest) = &self.manifest
            && path.starts_with(manifest.project_root())
            && manifest.allows_path(path)
        {
            return project_relative_path(&self.project_root, path)
                .map_or(PathScope::Standalone, PathScope::Project);
        }

        if let Some(root) = fallback_root_for_path(&self.fallback_roots, path)
            // The manifest root itself remains authoritative for exclusions;
            // a strictly nested workspace folder is an explicit fallback
            // authoring scope and may opt its files back in.
            && (!manifest_excluded || self.is_nested_fallback_root(root))
        {
            if recite_config::allows_unscoped_source_path(root, path) {
                let Some(candidate) = self.workspace_relative_path(path) else {
                    return PathScope::Standalone;
                };
                if let Some(existing) = self
                    .documents
                    .values()
                    .find(|document| document.identity.canonical_path == path)
                {
                    return PathScope::Project(existing.identity.project_relative_path.clone());
                }
                if self
                    .documents
                    .values()
                    .any(|document| document.identity.project_relative_path == candidate)
                {
                    return PathScope::Project(stable_path_identity(path));
                }
                return PathScope::Project(candidate);
            }
            if is_recite_path(path) {
                return PathScope::Excluded;
            }
        }

        if manifest_excluded && is_recite_path(path) {
            return PathScope::Excluded;
        }

        PathScope::Standalone
    }

    fn is_nested_fallback_root(&self, root: &Path) -> bool {
        self.manifest.as_ref().is_some_and(|manifest| {
            root.starts_with(manifest.project_root()) && root != manifest.project_root()
        })
    }

    fn workspace_relative_path(&self, path: &Path) -> Option<String> {
        if !self.workspace_root.as_os_str().is_empty() {
            project_relative_path(&self.workspace_root, path)
        } else {
            Some(stable_path_identity(path))
        }
    }
}

fn fallback_root_for_path<'a>(roots: &'a [PathBuf], path: &Path) -> Option<&'a Path> {
    roots
        .iter()
        .filter(|root| path.starts_with(root))
        .fold(None, |best, root| match best {
            Some(current) if current.components().count() >= root.components().count() => best,
            _ => Some(root.as_path()),
        })
}

fn is_recite_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "recite")
}
