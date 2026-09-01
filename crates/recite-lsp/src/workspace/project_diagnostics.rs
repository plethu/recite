use std::collections::BTreeMap;
use std::path::PathBuf;

use super::SavedProjectIndex;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManifestDiagnostics {
    pub(crate) path: PathBuf,
    pub(crate) text: String,
    pub(crate) diagnostics: Vec<recite_core::Diagnostic>,
}

impl SavedProjectIndex {
    pub(crate) fn manifest_diagnostics(&self) -> &BTreeMap<PathBuf, ManifestDiagnostics> {
        &self.manifest_diagnostics
    }

    pub(super) fn add_manifest_diagnostics(
        &mut self,
        report: &recite_config::ProjectDiscoveryReport,
    ) {
        self.add_manifest_diagnostics_value(
            report.manifest().manifest_path().to_owned(),
            report.manifest().source().source_text().to_owned(),
            report
                .diagnostics()
                .iter()
                .map(recite_config::DiscoveryDiagnostic::as_core_diagnostic)
                .collect(),
        );
    }

    pub(super) fn add_manifest_diagnostics_value(
        &mut self,
        path: PathBuf,
        text: String,
        diagnostics: Vec<recite_core::Diagnostic>,
    ) {
        if diagnostics.is_empty() {
            return;
        }
        self.manifest_diagnostics.insert(
            path.clone(),
            ManifestDiagnostics {
                path,
                text,
                diagnostics,
            },
        );
    }
}
