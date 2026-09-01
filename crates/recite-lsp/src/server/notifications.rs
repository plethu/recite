use lsp_server::Notification;
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Notification as LspNotification,
};
use lsp_types::{
    DidChangeTextDocumentParams, DidChangeWatchedFilesParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams,
};

use super::{Server, ServerError};
use crate::workspace::{DiagnosticRefresh, WorkspaceChangeResult};

impl Server {
    pub(super) fn handle_notification(
        &mut self,
        notification: Notification,
    ) -> Result<bool, ServerError> {
        match notification.method.as_str() {
            lsp_types::notification::Initialized::METHOD => {}
            lsp_types::notification::DidSaveTextDocument::METHOD => {
                self.handle_did_save(notification)?
            }
            lsp_types::notification::Exit::METHOD => {
                if self.shutdown_requested {
                    return Ok(true);
                }

                return Err(ServerError::ExitWithoutShutdown);
            }
            lsp_types::notification::DidOpenTextDocument::METHOD => {
                self.handle_did_open(notification)?
            }
            lsp_types::notification::DidChangeTextDocument::METHOD => {
                self.handle_did_change(notification)?
            }
            lsp_types::notification::DidChangeWatchedFiles::METHOD => {
                self.handle_did_change_watched_files(notification)?
            }
            lsp_types::notification::DidCloseTextDocument::METHOD => {
                self.handle_did_close(notification)?
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_did_open(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
        else {
            return Ok(());
        };
        let refreshes = self.workspace.open_refreshes(
            params.text_document.uri.clone(),
            params.text_document.version,
            params.text_document.text,
        );
        for refresh in refreshes {
            self.publish_refresh(refresh)?;
        }
        self.publish_open_document_refreshes(Some(&params.text_document.uri))
    }

    fn handle_did_change(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
        else {
            return Ok(());
        };
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        match self
            .workspace
            .change(uri.clone(), version, params.content_changes)
        {
            WorkspaceChangeResult::Accepted(refresh) => {
                self.publish_refresh(refresh)?;
                self.publish_open_document_refreshes(Some(&uri))?;
            }
            WorkspaceChangeResult::AcceptedRefreshes(refreshes) => {
                for refresh in refreshes {
                    self.publish_refresh(refresh)?;
                }
                self.publish_open_document_refreshes(Some(&uri))?;
            }
            _ => {}
        }

        Ok(())
    }

    fn handle_did_save(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidSaveTextDocumentParams>(DidSaveTextDocument::METHOD)
        else {
            return Ok(());
        };
        let uri = params.text_document.uri;
        let schema_refreshed = if let Some(refresh) = self.workspace.save_schema(&uri) {
            self.publish_refresh(refresh)?;
            self.publish_open_document_refreshes(None)?;
            true
        } else {
            false
        };
        if schema_refreshed {
            return Ok(());
        }
        for refresh in self.workspace.save(uri.clone()) {
            self.publish_refresh(refresh)?;
        }
        self.publish_open_document_refreshes(Some(&uri))?;

        Ok(())
    }

    fn handle_did_change_watched_files(
        &mut self,
        notification: Notification,
    ) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidChangeWatchedFilesParams>(DidChangeWatchedFiles::METHOD)
        else {
            return Ok(());
        };
        for event in params.changes {
            for refresh in self.workspace.refresh_watched_uri(&event.uri) {
                self.publish_refresh(refresh)?;
            }
            self.publish_open_document_refreshes(None)?;
        }
        Ok(())
    }

    fn handle_did_close(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
        else {
            return Ok(());
        };
        let refreshes = self.workspace.close(params.text_document.uri);
        let explicit_open_uri = refreshes.iter().find_map(|refresh| match refresh {
            DiagnosticRefresh::Publish(diagnostics) => Some(diagnostics.uri.clone()),
            DiagnosticRefresh::Clear { .. } => None,
        });
        for refresh in &refreshes {
            self.publish_refresh(refresh.clone())?;
        }
        if !refreshes.is_empty() {
            self.publish_open_document_refreshes(explicit_open_uri.as_ref())?;
        }
        Ok(())
    }
}
