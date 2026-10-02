use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceChangeResult};
use lsp_server::Notification;
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Notification as _,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, Uri,
};

#[derive(Clone)]
pub(super) enum Update {
    Open(DidOpenTextDocumentParams),
    Change(DidChangeTextDocumentParams),
    Save(DidSaveTextDocumentParams),
    Close(DidCloseTextDocumentParams),
    Watch(DidChangeWatchedFilesParams),
}
impl Update {
    pub(super) fn parse(notification: Notification) -> Option<Self> {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => notification
                .extract(DidOpenTextDocument::METHOD)
                .ok()
                .map(Self::Open),
            DidChangeTextDocument::METHOD => notification
                .extract(DidChangeTextDocument::METHOD)
                .ok()
                .map(Self::Change),
            DidSaveTextDocument::METHOD => notification
                .extract(DidSaveTextDocument::METHOD)
                .ok()
                .map(Self::Save),
            DidCloseTextDocument::METHOD => notification
                .extract(DidCloseTextDocument::METHOD)
                .ok()
                .map(Self::Close),
            DidChangeWatchedFiles::METHOD => notification
                .extract(DidChangeWatchedFiles::METHOD)
                .ok()
                .map(Self::Watch),
            _ => None,
        }
    }
    pub(super) fn changed_uri(&self) -> Option<&Uri> {
        match self {
            Self::Change(p) => Some(&p.text_document.uri),
            _ => None,
        }
    }
    pub(super) fn is_applied(&self, workspace: &LspWorkspace) -> bool {
        match self {
            Self::Open(p) => {
                workspace.open_version(&p.text_document.uri) == Some(p.text_document.version)
            }
            Self::Change(p) => {
                workspace.open_version(&p.text_document.uri) == Some(p.text_document.version)
            }
            Self::Close(p) => workspace.open_version(&p.text_document.uri).is_none(),
            _ => true,
        }
    }
    pub(super) fn apply(&self, workspace: &mut LspWorkspace) -> Vec<DiagnosticRefresh> {
        let mut refreshes = match self {
            Self::Open(p) => workspace.open_refreshes(
                p.text_document.uri.clone(),
                p.text_document.version,
                p.text_document.text.clone(),
            ),
            Self::Change(p) => match workspace.change(
                p.text_document.uri.clone(),
                p.text_document.version,
                p.content_changes.clone(),
            ) {
                WorkspaceChangeResult::Accepted(refresh) => vec![refresh],
                WorkspaceChangeResult::AcceptedRefreshes(refreshes) => refreshes,
                _ => Vec::new(),
            },
            Self::Save(p) => {
                if let Some(refresh) = workspace.save_schema(&p.text_document.uri) {
                    vec![refresh]
                } else {
                    workspace.save(p.text_document.uri.clone())
                }
            }
            Self::Close(p) => workspace.close(p.text_document.uri.clone()),
            Self::Watch(p) => p
                .changes
                .iter()
                .flat_map(|event| workspace.refresh_watched_uri(&event.uri))
                .collect(),
        };
        refreshes.extend(workspace.open_document_diagnostics_except(None));
        refreshes
    }
}
