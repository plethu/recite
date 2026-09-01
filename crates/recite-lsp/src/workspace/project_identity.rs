use std::path::Path;

use super::{SavedProjectIndex, WorkspaceDiscoveryState};
use crate::paths::{project_relative_path, stable_path_identity};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProjectIdentity {
    pub(crate) partition: String,
    pub(crate) key: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PathScope {
    Project(String),
    Excluded,
    Standalone,
}

impl SavedProjectIndex {
    pub(crate) fn project_key_for_path(&self, path: &Path) -> Option<String> {
        match self.project_identity_for_path(path) {
            Some(identity) => Some(identity.key),
            None => None,
        }
    }

    /// Resolve an open buffer's project identity, including a source-only
    /// identity for a root whose manifest could not be loaded. Failed
    /// manifests deliberately do not contribute saved documents, but an
    /// editor can still open and author a buffer below that explicit root.
    pub(crate) fn project_identity_for_open_path(&self, path: &Path) -> Option<ProjectIdentity> {
        if let Some(identity) = self.valid_manifest_identity(path) {
            return Some(identity);
        }
        let discovery = self.deepest_discovery(path)?;
        match &discovery.state {
            WorkspaceDiscoveryState::Failed { .. } => self.fallback_identity(&discovery.root, path),
            WorkspaceDiscoveryState::Manifestless => self.fallback_identity(&discovery.root, path),
            WorkspaceDiscoveryState::Manifest(report)
                if discovery.root != report.manifest().project_root() =>
            {
                self.fallback_identity(&discovery.root, path)
            }
            WorkspaceDiscoveryState::Manifest(_) => None,
        }
    }

    pub(crate) fn project_key_for_open_path(&self, path: &Path) -> Option<String> {
        self.project_identity_for_open_path(path)
            .map(|identity| identity.key)
    }

    pub(crate) fn partition_for_open_path(&self, path: &Path) -> Option<String> {
        self.project_identity_for_open_path(path)
            .map(|identity| identity.partition)
            .or_else(|| {
                self.project_identity_for_path(path)
                    .map(|identity| identity.partition)
            })
    }

    pub(crate) fn project_identity_for_path(&self, path: &Path) -> Option<ProjectIdentity> {
        if let Some(identity) = self.valid_manifest_identity(path) {
            return Some(identity);
        }
        let discovery = self.deepest_discovery(path)?;
        match &discovery.state {
            WorkspaceDiscoveryState::Manifestless => self.fallback_identity(&discovery.root, path),
            WorkspaceDiscoveryState::Manifest(report)
                if discovery.root != report.manifest().project_root() =>
            {
                self.fallback_identity(&discovery.root, path)
            }
            WorkspaceDiscoveryState::Failed { .. } | WorkspaceDiscoveryState::Manifest(_) => None,
        }
    }

    pub(crate) fn path_scope(&self, path: &Path) -> PathScope {
        if let Some(identity) = self.valid_manifest_identity(path) {
            return PathScope::Project(identity.key);
        }
        let Some(discovery) = self.deepest_discovery(path) else {
            return PathScope::Standalone;
        };
        match &discovery.state {
            WorkspaceDiscoveryState::Failed { .. } => self.fallback_scope(&discovery.root, path),
            WorkspaceDiscoveryState::Manifestless => self.fallback_scope(&discovery.root, path),
            WorkspaceDiscoveryState::Manifest(report) => {
                if discovery.root != report.manifest().project_root() {
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

    fn valid_manifest_identity(&self, path: &Path) -> Option<ProjectIdentity> {
        self.discoveries()
            .iter()
            .filter_map(|discovery| {
                let WorkspaceDiscoveryState::Manifest(report) = &discovery.state else {
                    return None;
                };
                let manifest = report.manifest();
                if !manifest.allows_path(path) {
                    return None;
                }
                let key = project_relative_path(manifest.project_root(), path)?;
                Some((
                    manifest.project_root().components().count(),
                    manifest.manifest_path(),
                    ProjectIdentity {
                        partition: stable_path_identity(manifest.project_root()),
                        key,
                    },
                ))
            })
            .max_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(right.1)))
            .map(|(_, _, identity)| identity)
    }

    fn fallback_identity(&self, root: &Path, path: &Path) -> Option<ProjectIdentity> {
        if !recite_config::allows_unscoped_source_path(root, path) {
            return None;
        }
        let key = self.workspace_relative_path(path)?;
        let key = if let Some(existing) = self
            .documents
            .values()
            .find(|document| document.identity.canonical_path == path)
        {
            existing.identity.project_relative_path.clone()
        } else if self
            .documents
            .values()
            .any(|document| document.identity.project_relative_path == key)
        {
            stable_path_identity(path)
        } else {
            key
        };
        Some(ProjectIdentity {
            partition: stable_path_identity(root),
            key,
        })
    }

    fn fallback_scope(&self, root: &Path, path: &Path) -> PathScope {
        self.fallback_identity(root, path)
            .map(|identity| PathScope::Project(identity.key))
            .unwrap_or_else(|| {
                if is_recite_path(path) {
                    PathScope::Excluded
                } else {
                    PathScope::Standalone
                }
            })
    }

    fn deepest_discovery(&self, path: &Path) -> Option<&super::WorkspaceDiscovery> {
        self.discoveries()
            .iter()
            .filter(|discovery| path.starts_with(&discovery.root))
            .max_by_key(|discovery| discovery.root.components().count())
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
