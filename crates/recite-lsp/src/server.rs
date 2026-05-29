use lsp_server::{Connection, ErrorCode, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Initialized, Notification as LspNotification, PublishDiagnostics,
};
use lsp_types::request::{Request as LspRequest, Shutdown};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeParams, InitializeResult, PositionEncodingKind, SaveOptions, ServerCapabilities,
    ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncOptions,
    TextDocumentSyncSaveOptions,
};

use crate::diagnostics::{clear_diagnostics, publish_diagnostics};
use crate::documents::OpenDocumentStore;

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

    let mut server = Server::new(connection);
    server.run()
}

fn initialize_result(params: &InitializeParams) -> InitializeResult {
    InitializeResult {
        capabilities: ServerCapabilities {
            position_encoding: Some(select_position_encoding(params)),
            text_document_sync: Some(TextDocumentSyncCapability::Options(
                TextDocumentSyncOptions {
                    open_close: Some(true),
                    change: Some(TextDocumentSyncKind::FULL),
                    will_save: None,
                    will_save_wait_until: None,
                    save: Some(TextDocumentSyncSaveOptions::SaveOptions(SaveOptions {
                        include_text: None,
                    })),
                },
            )),
            ..ServerCapabilities::default()
        },
        server_info: Some(ServerInfo {
            name: "recite-lsp".to_owned(),
            version: Some(env!("CARGO_PKG_VERSION").to_owned()),
        }),
    }
}

fn select_position_encoding(_params: &InitializeParams) -> PositionEncodingKind {
    // UTF-16 is mandatory in LSP 3.17. If the client omits it from the
    // advertised list, the server may still assume support.
    PositionEncodingKind::UTF16
}

struct Server {
    connection: Connection,
    documents: OpenDocumentStore,
    shutdown_requested: bool,
}

impl Server {
    fn new(connection: Connection) -> Self {
        Self {
            connection,
            documents: OpenDocumentStore::default(),
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
            Initialized::METHOD | DidSaveTextDocument::METHOD => {}
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

    fn handle_did_open(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidOpenTextDocumentParams>(DidOpenTextDocument::METHOD)
        else {
            return Ok(());
        };
        let uri = params.text_document.uri;
        let publish_params = {
            let document = self.documents.open(
                uri.clone(),
                params.text_document.version,
                params.text_document.text,
            );
            publish_diagnostics(
                uri,
                document.text(),
                Some(document.version()),
                document.diagnostics(),
            )
        };
        self.send(Notification::new(PublishDiagnostics::METHOD.to_owned(), publish_params).into())
    }

    fn handle_did_change(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidChangeTextDocumentParams>(DidChangeTextDocument::METHOD)
        else {
            return Ok(());
        };
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        if let Some(document) = self.documents.change(&uri, version, params.content_changes) {
            let publish_params = publish_diagnostics(
                uri,
                document.text(),
                Some(document.version()),
                document.diagnostics(),
            );
            self.send(
                Notification::new(PublishDiagnostics::METHOD.to_owned(), publish_params).into(),
            )?;
        }

        Ok(())
    }

    fn handle_did_close(&mut self, notification: Notification) -> Result<(), ServerError> {
        let Ok(params) =
            notification.extract::<DidCloseTextDocumentParams>(DidCloseTextDocument::METHOD)
        else {
            return Ok(());
        };
        if self.documents.close(&params.text_document.uri) {
            self.publish_clear(params.text_document.uri)?;
        }

        Ok(())
    }

    fn publish_clear(&self, uri: lsp_types::Uri) -> Result<(), ServerError> {
        self.send(
            Notification::new(
                PublishDiagnostics::METHOD.to_owned(),
                clear_diagnostics(uri),
            )
            .into(),
        )
    }

    fn send(&self, message: Message) -> Result<(), ServerError> {
        self.connection
            .sender
            .send(message)
            .map_err(|_| ServerError::Send)
    }
}
