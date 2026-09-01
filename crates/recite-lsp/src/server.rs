use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidChangeWatchedFiles, DidCloseTextDocument, DidOpenTextDocument,
    DidSaveTextDocument, Exit, Initialized, LogMessage, Notification as LspNotification,
    PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Completion, GotoDefinition, HoverRequest, PrepareRenameRequest, References,
    Rename, Request as LspRequest, Shutdown,
};
use lsp_types::{
    CodeActionParams, CompletionParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    GotoDefinitionParams, HoverParams, LogMessageParams, MessageType, ReferenceParams,
    RenameParams, TextDocumentPositionParams,
};

use crate::diagnostics::{clear_diagnostics, publish_diagnostics};
use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};
use recite_ui::UiCatalog;

mod bootstrap;
mod error;
#[allow(unused_imports, reason = "used by in-crate lifecycle harness")]
pub(crate) use bootstrap::run_connection_with_user_config;
#[allow(unused_imports, reason = "used by in-crate protocol harness")]
pub(crate) use bootstrap::{run_connection, run_connection_with_catalog};
pub use bootstrap::{run_stdio, run_stdio_with_catalog, run_stdio_with_locale};
pub use error::ServerError;

struct Server {
    connection: Connection,
    workspace: LspWorkspace,
    shutdown_requested: bool,
}

impl Server {
    fn new(
        connection: Connection,
        workspace_config: WorkspaceConfig,
        catalog: UiCatalog,
    ) -> Result<Self, ServerError> {
        Ok(Self {
            connection,
            workspace: LspWorkspace::with_ui_catalog(workspace_config, catalog)
                .map_err(|error| ServerError::Authoring(error.to_string()))?,
            shutdown_requested: false,
        })
    }

    fn run(&mut self) -> Result<(), ServerError> {
        while let Ok(message) = self.connection.receiver.recv() {
            match message {
                Message::Request(request) => {
                    if self.handle_request(request)? {
                        return Ok(());
                    }
                }
                Message::Notification(notification) => {
                    if self.handle_notification(notification)? {
                        return Ok(());
                    }
                }
                Message::Response(_) => {}
            }
        }

        if self.shutdown_requested {
            Ok(())
        } else {
            Err(ServerError::Disconnected)
        }
    }

    fn handle_request(&mut self, request: Request) -> Result<bool, ServerError> {
        if request.method == Shutdown::METHOD {
            let response = Response::new_ok(request.id, ());
            self.send(response.into())?;
            self.shutdown_requested = true;
            return Ok(false);
        }
        if request.method == Completion::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<CompletionParams>(Completion::METHOD) {
                Ok((_, params)) => self.workspace.completion(
                    &params.text_document_position.text_document.uri,
                    params.text_document_position.position,
                ),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == CodeActionRequest::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<CodeActionParams>(CodeActionRequest::METHOD) {
                Ok((_, params)) => self.workspace.code_action(&params),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == HoverRequest::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<HoverParams>(HoverRequest::METHOD) {
                Ok((_, params)) => self.workspace.hover(
                    &params.text_document_position_params.text_document.uri,
                    params.text_document_position_params.position,
                ),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == GotoDefinition::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<GotoDefinitionParams>(GotoDefinition::METHOD) {
                Ok((_, params)) => self.workspace.definition(
                    &params.text_document_position_params.text_document.uri,
                    params.text_document_position_params.position,
                ),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == References::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<ReferenceParams>(References::METHOD) {
                Ok((_, params)) => self.workspace.references(
                    &params.text_document_position.text_document.uri,
                    params.text_document_position.position,
                    params.context.include_declaration,
                ),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == PrepareRenameRequest::METHOD {
            let id = request.id.clone();
            let result = match request
                .extract::<TextDocumentPositionParams>(PrepareRenameRequest::METHOD)
            {
                Ok((_, params)) => self
                    .workspace
                    .prepare_rename(&params.text_document.uri, params.position),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        if request.method == Rename::METHOD {
            let id = request.id.clone();
            let result = match request.extract::<RenameParams>(Rename::METHOD) {
                Ok((_, params)) => self.workspace.rename(
                    &params.text_document_position.text_document.uri,
                    params.text_document_position.position,
                    &params.new_name,
                ),
                Err(error) => {
                    let response =
                        Response::new_err(id, ErrorCode::InvalidParams as i32, error.to_string());
                    self.send(response.into())?;
                    return Ok(false);
                }
            };
            self.send(Response::new_ok(id, result).into())?;
            return Ok(false);
        }
        let response = Response::new_err(
            request.id,
            ErrorCode::MethodNotFound as i32,
            format!("unsupported request method {}", request.method),
        );
        self.send(response.into())?;
        Ok(false)
    }
    fn handle_notification(&mut self, notification: Notification) -> Result<bool, ServerError> {
        match notification.method.as_str() {
            Initialized::METHOD => {}
            DidSaveTextDocument::METHOD => self.handle_did_save(notification)?,
            Exit::METHOD => {
                if self.shutdown_requested {
                    return Ok(true);
                }

                return Err(ServerError::ExitWithoutShutdown);
            }
            DidOpenTextDocument::METHOD => self.handle_did_open(notification)?,
            DidChangeTextDocument::METHOD => self.handle_did_change(notification)?,
            DidChangeWatchedFiles::METHOD => self.handle_did_change_watched_files(notification)?,
            DidCloseTextDocument::METHOD => self.handle_did_close(notification)?,
            _ => {}
        }
        Ok(false)
    }

    fn publish_schema_diagnostics(&mut self) -> Result<(), ServerError> {
        for refresh in self.workspace.project_diagnostics_all() {
            self.publish_refresh(refresh)?;
        }
        for refresh in self.workspace.schema_diagnostics_all() {
            self.publish_refresh(refresh)?;
        }

        Ok(())
    }
    fn publish_startup_warning(&self, message: String) -> Result<(), ServerError> {
        self.send(
            Notification::new(
                LogMessage::METHOD.to_owned(),
                LogMessageParams {
                    typ: MessageType::WARNING,
                    message,
                },
            )
            .into(),
        )
    }
    fn handle_did_open(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
        else {
            return Ok(());
        };
        let Some(refresh) = self.workspace.open(
            params.text_document.uri.clone(),
            params.text_document.version,
            params.text_document.text,
        ) else {
            return Ok(());
        };
        self.publish_refresh(refresh)?;
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
        if let WorkspaceChangeResult::Accepted(refresh) =
            self.workspace
                .change(uri.clone(), version, params.content_changes)
        {
            self.publish_refresh(refresh)?;
            self.publish_open_document_refreshes(Some(&uri))?;
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
    fn publish_refresh(&self, refresh: DiagnosticRefresh) -> Result<(), ServerError> {
        if !self.workspace.is_current_generation(refresh.generation()) {
            return Ok(());
        }

        match refresh {
            DiagnosticRefresh::Publish(diagnostics) => {
                let crate::workspace::DocumentDiagnostics {
                    uri,
                    text,
                    version,
                    diagnostics,
                    ..
                } = diagnostics;
                let sources = self.workspace.diagnostic_sources_for_uri(&uri);
                let publish_params = publish_diagnostics(
                    uri,
                    text.as_str(),
                    version,
                    &diagnostics,
                    &self.workspace.ui_catalog,
                    &sources,
                )
                .map_err(|error| ServerError::Diagnostics(error.to_string()))?;
                self.send(
                    Notification::new(PublishDiagnostics::METHOD.to_owned(), publish_params).into(),
                )
            }
            DiagnosticRefresh::Clear { uri, version, .. } => self.send(
                Notification::new(
                    PublishDiagnostics::METHOD.to_owned(),
                    clear_diagnostics(uri, version),
                )
                .into(),
            ),
        }
    }
    fn publish_open_document_refreshes(
        &self,
        exclude: Option<&lsp_types::Uri>,
    ) -> Result<(), ServerError> {
        for refresh in self.workspace.open_document_diagnostics_except(exclude) {
            self.publish_refresh(refresh)?;
        }

        Ok(())
    }
    fn send(&self, message: Message) -> Result<(), ServerError> {
        self.connection
            .sender
            .send(message)
            .map_err(|_| ServerError::Send)
    }
}
