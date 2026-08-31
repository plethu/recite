use std::collections::BTreeMap;
use std::path::PathBuf;

use super::{SavedProjectIndex, WorkspaceDiscoveryState};

#[derive(Clone, Debug)]
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

    pub(super) fn set_primary_manifest(&mut self) {
        let primary = self
            .manifest_diagnostics
            .values()
            .next()
            .map(|entry| (entry.path.clone(), entry.text.clone()));
        if let Some((path, text)) = primary {
            self.manifest_path = Some(path);
            self.manifest_text = text;
        } else if let Some(report) =
            self.discoveries
                .iter()
                .filter_map(|discovery| match &discovery.state {
                    WorkspaceDiscoveryState::Manifest(report) => Some(report),
                    WorkspaceDiscoveryState::Manifestless
                    | WorkspaceDiscoveryState::Failed { .. } => None,
                })
                .min_by_key(|report| report.manifest().manifest_path().to_owned())
        {
            self.manifest_path = Some(report.manifest().manifest_path().to_owned());
            self.manifest_text = report.manifest().source().source_text().to_owned();
        }
    }
}
