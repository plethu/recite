use std::path::{Path, PathBuf};

use super::SavedProjectIndex;
use crate::paths::{project_relative_path, stable_path_identity};

impl SavedProjectIndex {
    pub(crate) fn project_key_for_path(&self, path: &Path) -> Option<String> {
        if let Some(manifest) = &self.manifest
            && path.starts_with(manifest.project_root())
        {
            return manifest
                .allows_path(path)
                .then(|| project_relative_path(&self.project_root, path))
                .flatten();
        }

        if let Some(root) = self.fallback_root_for_path(path)
            && recite_config::allows_unscoped_source_path(root, path)
        {
            let candidate = self.workspace_relative_path(path)?;
            if let Some(existing) = self
                .documents
                .values()
                .find(|document| document.identity.canonical_path == path)
            {
                return Some(existing.identity.project_relative_path.clone());
            }
            if self
                .documents
                .values()
                .any(|document| document.identity.project_relative_path == candidate)
            {
                return Some(stable_path_identity(path));
            }
            return Some(candidate);
        }

        None
    }

    fn fallback_root_for_path(&self, path: &Path) -> Option<&Path> {
        self.fallback_roots
            .iter()
            .find(|root| path.starts_with(root))
            .map(PathBuf::as_path)
    }

    pub(crate) fn is_excluded_path(&self, path: &Path) -> bool {
        self.manifest.as_ref().is_some_and(|manifest| {
            path.starts_with(manifest.project_root()) && !manifest.allows_path(path)
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
