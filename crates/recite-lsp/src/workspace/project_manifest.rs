use std::path::Path;

use super::SavedProjectIndex;

impl SavedProjectIndex {
    /// Re-discover every explicitly configured root and replace the aggregate
    /// index in one operation. A sibling's failure therefore cannot erase or
    /// hide another root's documents, schema, or diagnostics.
    pub(crate) fn refresh_manifests(&mut self) -> Option<std::path::PathBuf> {
        let discoveries = super::super::config::discover_workspace_roots(&self.fallback_roots);
        let schema_path = discoveries
            .iter()
            .filter_map(|discovery| match &discovery.state {
                super::super::config::WorkspaceDiscoveryState::Manifest(report) => Some(report),
                super::super::config::WorkspaceDiscoveryState::Manifestless
                | super::super::config::WorkspaceDiscoveryState::Failed { .. } => None,
            })
            .min_by_key(|report| report.manifest().manifest_path().to_owned())
            .and_then(|report| super::super::config::schema_path_for_discovery(report));
        *self = Self::from_discoveries(self.fallback_roots.clone(), discoveries);
        schema_path
    }

    pub(crate) fn is_manifest_candidate(&self, path: &Path) -> bool {
        if path.file_name().and_then(|name| name.to_str())
            != Some(recite_config::PROJECT_MANIFEST_FILE)
        {
            return false;
        }
        self.fallback_roots.iter().any(|root| {
            path.starts_with(root) || path.parent().is_some_and(|parent| root.starts_with(parent))
        })
    }
}
