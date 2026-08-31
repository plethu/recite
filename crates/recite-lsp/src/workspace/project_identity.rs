use std::path::Path;

use super::{SavedProjectIndex, WorkspaceDiscoveryState};
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
        let Some(discovery) = self.deepest_discovery(path) else {
            return PathScope::Standalone;
        };
        match &discovery.state {
            WorkspaceDiscoveryState::Failed { .. } => PathScope::Standalone,
            WorkspaceDiscoveryState::Manifestless => self.fallback_scope(&discovery.root, path),
            WorkspaceDiscoveryState::Manifest(report) => {
                let manifest = report.manifest();
                if path.starts_with(manifest.project_root()) && manifest.allows_path(path) {
                    return project_relative_path(manifest.project_root(), path)
                        .map_or(PathScope::Standalone, PathScope::Project);
                }
                if discovery.root != manifest.project_root() {
                    return self.fallback_scope(&discovery.root, path);
                }
                if is_recite_path(path) {
                    PathScope::Excluded
                } else {
                    PathScope::Standalone
                }
            }
        }
    }

    fn deepest_discovery(&self, path: &Path) -> Option<&super::WorkspaceDiscovery> {
        self.discoveries()
            .iter()
            .filter(|discovery| path.starts_with(&discovery.root))
            .max_by_key(|discovery| discovery.root.components().count())
    }

    fn fallback_scope(&self, root: &Path, path: &Path) -> PathScope {
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
            PathScope::Excluded
        } else {
            PathScope::Standalone
        }
    }

    fn workspace_relative_path(&self, path: &Path) -> Option<String> {
        if !self.workspace_root.as_os_str().is_empty() {
            project_relative_path(&self.workspace_root, path)
        } else {
            Some(stable_path_identity(path))
        }
    }
}

fn is_recite_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension == "recite")
}
