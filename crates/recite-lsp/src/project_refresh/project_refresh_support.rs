use std::collections::BTreeMap;

use super::super::schema_index::SchemaIndex;
use super::super::{DiagnosticRefresh, LspWorkspace};

pub(super) fn coalesce_refreshes(refreshes: Vec<DiagnosticRefresh>) -> Vec<DiagnosticRefresh> {
    let mut coalesced = Vec::new();
    for refresh in refreshes {
        let uri = match &refresh {
            DiagnosticRefresh::Publish(published) => &published.uri,
            DiagnosticRefresh::Clear { uri, .. } => uri,
        };
        let Some(existing) = coalesced.iter_mut().find(|existing| {
            let existing_uri = match existing {
                DiagnosticRefresh::Publish(published) => &published.uri,
                DiagnosticRefresh::Clear { uri, .. } => uri,
            };
            existing_uri == uri
        }) else {
            coalesced.push(refresh);
            continue;
        };
        match (existing, refresh) {
            (DiagnosticRefresh::Publish(existing), DiagnosticRefresh::Publish(incoming)) => {
                existing.version = existing.version.or(incoming.version);
                if existing.text.is_empty() {
                    existing.text = incoming.text;
                }
                for diagnostic in incoming.diagnostics {
                    if !existing.diagnostics.contains(&diagnostic) {
                        existing.diagnostics.push(diagnostic);
                    }
                }
            }
            (slot @ DiagnosticRefresh::Clear { .. }, DiagnosticRefresh::Publish(incoming)) => {
                *slot = DiagnosticRefresh::Publish(incoming);
            }
            (DiagnosticRefresh::Publish(_), DiagnosticRefresh::Clear { .. })
            | (DiagnosticRefresh::Clear { .. }, DiagnosticRefresh::Clear { .. }) => {}
        }
    }
    coalesced
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
    let protocol_uri = schema.protocol_uri();
    documents
        .documents()
        .filter(|document| schema.matches_uri(&document.identity().uri))
        .map(|document| {
            let mut refresh =
                DiagnosticRefresh::publish_open(document, Vec::new(), workspace.generation);
            if let Some(protocol_uri) = &protocol_uri
                && let DiagnosticRefresh::Publish(published) = &mut refresh
            {
                published.uri = protocol_uri.clone();
            }
            refresh
        })
        .collect()
}
