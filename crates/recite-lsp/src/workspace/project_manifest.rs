use std::fs;
use std::path::{Path, PathBuf};

use super::SavedProjectIndex;

impl SavedProjectIndex {
    /// Re-read the manifest and replace the saved project state atomically.
    /// A failed manifest leaves no saved documents; callers may still layer
    /// open editor buffers on top of the diagnostic-only state.
    pub(crate) fn refresh_manifest(&mut self) -> Option<PathBuf> {
        let start = self.discovery_start.clone()?;
        let result = recite_config::discover_project(start);
        let schema_path = result
            .as_ref()
            .ok()
            .and_then(super::super::config::schema_path_for_discovery);
        self.apply_discovery(result);
        schema_path
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
                super::append_unique_paths(&mut self.roots, &self.fallback_roots);
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
                let fallback_roots = self.fallback_roots[1..].to_vec();
                self.insert_fallback_documents(&fallback_roots);
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

    pub(crate) fn is_manifest_candidate(&self, path: &Path) -> bool {
        if path.file_name().and_then(|name| name.to_str())
            != Some(recite_config::PROJECT_MANIFEST_FILE)
        {
            return false;
        }
        let Some(start) = self.discovery_start.as_deref() else {
            return false;
        };
        let Some(mut directory) = (if start.is_dir() {
            Some(start.to_owned())
        } else {
            start.parent().map(Path::to_owned)
        }) else {
            return false;
        };
        loop {
            if directory.join(recite_config::PROJECT_MANIFEST_FILE) == path {
                return true;
            }
            if !directory.pop() {
                return false;
            }
        }
    }
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
