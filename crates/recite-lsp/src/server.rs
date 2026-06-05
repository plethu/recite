use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Initialized, Notification as LspNotification, PublishDiagnostics,
};
use lsp_types::request::{Completion, HoverRequest, Request as LspRequest, Shutdown};
use lsp_types::{
    CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, HoverParams, InitializeParams,
};

use crate::capabilities::initialize_result;
use crate::diagnostics::{clear_diagnostics, publish_diagnostics};
use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    #[error("LSP protocol error: {0}")]
    Protocol(#[from] lsp_server::ProtocolError),
    #[error("LSP transport disconnected")]
    Disconnected,
    #[error("client exited before shutdown")]
    ExitWithoutShutdown,
    #[error("failed to send LSP message")]
    Send,
    #[error("failed to join LSP stdio threads: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize initialize result: {0}")]
    InitializeResult(#[from] serde_json::Error),
}

pub fn run_stdio() -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    run_connection(connection)?;
    io_threads.join()?;

    Ok(())
}

pub(crate) fn run_connection(connection: Connection) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params = serde_json::from_value::<InitializeParams>(initialize_params)
        .unwrap_or_else(|_| InitializeParams::default());
    let initialize_result = initialize_result(&initialize_params);
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;

    let mut server = Server::new(
        connection,
        WorkspaceConfig::from_initialize_params(&initialize_params),
    );
    server.publish_schema_diagnostics()?;
    server.run()
}

struct Server {
    connection: Connection,
    workspace: LspWorkspace,
    shutdown_requested: bool,
}

impl Server {
    fn new(connection: Connection, workspace_config: WorkspaceConfig) -> Self {
        Self {
            connection,
            workspace: LspWorkspace::new(workspace_config),
            shutdown_requested: false,
        }
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
            DidCloseTextDocument::METHOD => self.handle_did_close(notification)?,
            _ => {}
        }

        Ok(false)
    }

    fn publish_schema_diagnostics(&mut self) -> Result<(), ServerError> {
        if let Some(refresh) = self.workspace.schema_diagnostics() {
            self.publish_refresh(refresh)?;
        }

        Ok(())
    }

    fn handle_did_open(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
        else {
            return Ok(());
        };
        let refresh = self.workspace.open(
            params.text_document.uri.clone(),
            params.text_document.version,
            params.text_document.text,
        );
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
        if let Some(refresh) = self.workspace.save_schema(&uri) {
            self.publish_refresh(refresh)?;
            self.publish_open_document_refreshes(None)?;
        }
        if let Some(refresh) = self.workspace.save(uri.clone()) {
            self.publish_refresh(refresh)?;
            self.publish_open_document_refreshes(Some(&uri))?;
        }

        Ok(())
    }

    fn handle_did_close(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
        else {
            return Ok(());
        };
        if let Some(refresh) = self.workspace.close(params.text_document.uri) {
            self.publish_refresh(refresh)?;
            self.publish_open_document_refreshes(None)?;
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
                } = self.workspace.with_semantic_diagnostics(diagnostics);
                let publish_params = publish_diagnostics(uri, text.as_str(), version, &diagnostics);
                self.send(
                    Notification::new(PublishDiagnostics::METHOD.to_owned(), publish_params).into(),
                )
            }
            DiagnosticRefresh::Clear { uri, .. } => self.send(
                Notification::new(
                    PublishDiagnostics::METHOD.to_owned(),
                    clear_diagnostics(uri),
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
