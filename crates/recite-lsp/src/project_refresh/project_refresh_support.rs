use std::collections::BTreeMap;

use super::super::schema_index::SchemaIndex;
use super::super::{DiagnosticRefresh, LspWorkspace};

pub(super) fn schema_paths_for_saved(
    saved: &super::super::project_index::SavedProjectIndex,
    override_path: Option<&std::path::PathBuf>,
) -> BTreeMap<String, Option<std::path::PathBuf>> {
    saved
        .partition_ids()
        .into_iter()
        .map(|id| {
            let path = override_path.cloned().or_else(|| {
                saved
                    .discoveries()
                    .iter()
                    .find_map(|discovery| match &discovery.state {
                        super::super::config::WorkspaceDiscoveryState::Manifest(report)
                            if crate::paths::stable_path_identity(
                                report.manifest().project_root(),
                            ) == id =>
                        {
                            super::super::config::schema_path_for_discovery(report)
                        }
                        _ => None,
                    })
            });
            (id, path)
        })
        .collect()
}

pub(super) fn manifest_refreshes(
    workspace: &LspWorkspace,
    old: &BTreeMap<
        std::path::PathBuf,
        super::super::project_index::project_diagnostics::ManifestDiagnostics,
    >,
) -> Vec<DiagnosticRefresh> {
    let mut refreshes = Vec::new();
    for entry in workspace.saved.manifest_diagnostics().values() {
        let changed = old.get(&entry.path).is_none_or(|previous| {
            previous.text != entry.text || previous.diagnostics != entry.diagnostics
        });
        if changed && let Some(uri) = crate::paths::file_path_to_uri(&entry.path) {
            refreshes.push(DiagnosticRefresh::Publish(
                super::super::DocumentDiagnostics {
                    uri,
                    text: entry.text.clone(),
                    version: None,
                    diagnostics: entry.diagnostics.clone(),
                    generation: workspace.generation,
                },
            ));
        }
    }
    for entry in old.values() {
        if !workspace
            .saved
            .manifest_diagnostics()
            .contains_key(&entry.path)
            && let Some(uri) = crate::paths::file_path_to_uri(&entry.path)
        {
            refreshes.push(DiagnosticRefresh::Clear {
                uri,
                version: None,
                generation: workspace.generation,
            });
        }
    }
    refreshes
}

pub(super) fn clear_old_schema(
    workspace: &LspWorkspace,
    schema: &SchemaIndex,
    documents: &crate::documents::OpenDocumentStore,
) -> Vec<DiagnosticRefresh> {
    documents
        .documents()
        .filter(|document| schema.matches_uri(&document.identity().uri))
        .map(|document| DiagnosticRefresh::publish_open(document, Vec::new(), workspace.generation))
        .collect()
}
