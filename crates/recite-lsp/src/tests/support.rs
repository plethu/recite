use std::fs;
use std::path::Path;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Exit,
    Initialized, Notification as LspNotification, PublishDiagnostics,
};
use lsp_types::request::{
    CodeActionRequest, Completion, GotoDefinition, HoverRequest, PrepareRenameRequest, References,
    Rename, Request as LspRequest, Shutdown,
};
use lsp_types::{CodeActionParams, CodeActionResponse, RenameParams};
use lsp_types::{
    CompletionParams, CompletionResponse, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
    DidOpenTextDocumentParams, DidSaveTextDocumentParams, GotoDefinitionParams,
    GotoDefinitionResponse, Hover, HoverParams, InitializeResult, Location, PartialResultParams,
    Position, PrepareRenameResponse, PublishDiagnosticsParams, ReferenceContext, ReferenceParams,
    TextDocumentContentChangeEvent, TextDocumentIdentifier, TextDocumentItem,
    TextDocumentPositionParams, Uri, VersionedTextDocumentIdentifier, WorkDoneProgressParams,
    WorkspaceEdit,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::server::{ServerError, run_connection};
use crate::workspace::LspWorkspace;

pub(super) struct Harness {
    client: Connection,
    server: JoinHandle<Result<(), ServerError>>,
    next_id: i32,
}

impl Harness {
    pub(super) fn start() -> Self {
        Self::start_with_result(json!({
            "capabilities": {
                "general": {
                    "positionEncodings": ["utf-8", "utf-16"]
                }
            }
        }))
        .0
    }

    pub(super) fn start_with_result(params: Value) -> (Self, InitializeResult) {
        let (server_connection, client) = Connection::memory();
        let server = thread::spawn(move || run_connection(server_connection));
        let mut harness = Self {
            client,
            server,
            next_id: 1,
        };
        let result = harness.initialize(params);
        (harness, result)
    }

    pub(super) fn send_notification(&self, method: &str, params: impl Serialize) {
        self.send(Message::Notification(Notification {
            method: method.to_owned(),
            params: to_value(params),
        }));
    }

    pub(super) fn recv_publish_diagnostics(&self) -> PublishDiagnosticsParams {
        match self.client.receiver.recv_timeout(Duration::from_secs(1)) {
            Ok(Message::Notification(notification)) => {
                assert_eq!(notification.method, PublishDiagnostics::METHOD);
                from_value(notification.params)
            }
            Ok(other) => panic!("expected diagnostics notification, got {other:?}"),
            Err(error) => panic!("timed out or failed waiting for diagnostics: {error}"),
        }
    }

    pub(super) fn assert_no_message(&self) {
        for _ in 0..10 {
            if let Ok(message) = self.client.receiver.try_recv() {
                panic!("expected no server message, got {message:?}");
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    pub(super) fn did_open(&self, uri: Uri, version: i32, text: &str) {
        self.send_notification(
            DidOpenTextDocument::METHOD,
            DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri,
                    language_id: "recite".to_owned(),
                    version,
                    text: text.to_owned(),
                },
            },
        );
    }

    pub(super) fn did_change(
        &self,
        uri: Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) {
        self.send_notification(
            DidChangeTextDocument::METHOD,
            DidChangeTextDocumentParams {
                text_document: VersionedTextDocumentIdentifier { uri, version },
                content_changes: changes,
            },
        );
    }

    pub(super) fn did_close(&self, uri: Uri) {
        self.send_notification(
            DidCloseTextDocument::METHOD,
            DidCloseTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
            },
        );
    }

    pub(super) fn did_save(&self, uri: Uri) {
        self.send_notification(
            DidSaveTextDocument::METHOD,
            DidSaveTextDocumentParams {
                text_document: TextDocumentIdentifier { uri },
                text: None,
            },
        );
    }

    pub(super) fn completion(
        &mut self,
        uri: Uri,
        position: Position,
    ) -> Option<CompletionResponse> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: Completion::METHOD.to_owned(),
            params: to_value(CompletionParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: None,
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn hover(&mut self, uri: Uri, position: Position) -> Option<Hover> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: HoverRequest::METHOD.to_owned(),
            params: to_value(HoverParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn definition(
        &mut self,
        uri: Uri,
        position: Position,
    ) -> Option<GotoDefinitionResponse> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: GotoDefinition::METHOD.to_owned(),
            params: to_value(GotoDefinitionParams {
                text_document_position_params: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn references(
        &mut self,
        uri: Uri,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: References::METHOD.to_owned(),
            params: to_value(ReferenceParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position,
                },
                work_done_progress_params: WorkDoneProgressParams::default(),
                partial_result_params: PartialResultParams::default(),
                context: ReferenceContext {
                    include_declaration,
                },
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn prepare_rename(
        &mut self,
        uri: Uri,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: PrepareRenameRequest::METHOD.to_owned(),
            params: to_value(TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri },
                position,
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn rename(
        &mut self,
        uri: Uri,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: Rename::METHOD.to_owned(),
            params: to_value(RenameParams {
                text_document_position: TextDocumentPositionParams {
                    text_document: TextDocumentIdentifier { uri },
                    position,
                },
                new_name: new_name.to_owned(),
                work_done_progress_params: WorkDoneProgressParams::default(),
            }),
        }));
        self.recv_response_result()
    }

    pub(super) fn code_action(&mut self, params: CodeActionParams) -> Option<CodeActionResponse> {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: CodeActionRequest::METHOD.to_owned(),
            params: to_value(params),
        }));
        self.recv_response_result()
    }

    pub(super) fn raw_request_response(&mut self, method: &str, params: Value) -> Response {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: method.to_owned(),
            params,
        }));
        self.recv_response()
    }

    pub(super) fn finish(self) {
        let mut this = self;
        let id = this.next_request_id();
        this.send(Message::Request(Request {
            id,
            method: Shutdown::METHOD.to_owned(),
            params: Value::Null,
        }));
        let response = this.recv_response();
        assert!(response.error.is_none());
        this.send_notification(Exit::METHOD, ());

        match this.server.join() {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("server returned error: {error}"),
            Err(_) => panic!("server thread panicked"),
        }
    }

    pub(super) fn exit_without_shutdown(self) -> Result<(), ServerError> {
        self.send_notification(Exit::METHOD, ());
        match self.server.join() {
            Ok(result) => result,
            Err(_) => panic!("server thread panicked"),
        }
    }

    fn initialize(&mut self, params: Value) -> InitializeResult {
        let id = self.next_request_id();
        self.send(Message::Request(Request {
            id,
            method: "initialize".to_owned(),
            params,
        }));
        let response = self.recv_response();
        let result = match response.result {
            Some(result) => result,
            None => panic!("initialize response did not include a result"),
        };
        let result = from_value::<InitializeResult>(result);
        self.send_notification(Initialized::METHOD, ());
        result
    }

    fn next_request_id(&mut self) -> RequestId {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        RequestId::from(id)
    }

    fn send(&self, message: Message) {
        if let Err(error) = self.client.sender.send(message) {
            panic!("failed to send client message: {error}");
        }
    }

    fn recv_response(&self) -> Response {
        match self.client.receiver.recv() {
            Ok(Message::Response(response)) => response,
            Ok(other) => panic!("expected response, got {other:?}"),
            Err(error) => panic!("failed to receive response: {error}"),
        }
    }

    fn recv_response_result<T: serde::de::DeserializeOwned>(&self) -> T {
        let response = self.recv_response();
        if let Some(error) = response.error {
            panic!("request failed: {error:?}");
        }
        from_value(response.result.unwrap_or(Value::Null))
    }
}

pub(super) fn full_change(text: &str) -> TextDocumentContentChangeEvent {
    TextDocumentContentChangeEvent {
        range: None,
        range_length: None,
        text: text.to_owned(),
    }
}

pub(super) fn uri(value: &str) -> Uri {
    match value.parse::<Uri>() {
        Ok(uri) => uri,
        Err(error) => panic!("invalid test URI {value}: {error}"),
    }
}

pub(super) fn file_uri(path: &Path) -> Uri {
    match crate::paths::file_path_to_uri(path) {
        Some(uri) => uri,
        None => panic!(
            "path cannot be represented as a file URI: {}",
            path.display()
        ),
    }
}

pub(super) fn harness_for_root(root: &Path) -> Harness {
    let root_uri = file_uri(root);
    Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        },
        "rootUri": root_uri.as_str()
    }))
    .0
}

pub(super) fn block_names(workspace: &LspWorkspace) -> Vec<String> {
    workspace
        .snapshot()
        .summaries()
        .iter()
        .flat_map(|summary| summary.blocks.iter().map(|block| block.name.clone()))
        .collect()
}

pub(super) fn write_file(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent()
        && let Err(error) = fs::create_dir_all(parent)
    {
        panic!("failed to create {}: {error}", parent.display());
    }
    if let Err(error) = fs::write(&path, contents) {
        panic!("failed to write {}: {error}", path.display());
    }
}

fn to_value(value: impl Serialize) -> Value {
    match serde_json::to_value(value) {
        Ok(value) => value,
        Err(error) => panic!("failed to serialize test value: {error}"),
    }
}

fn from_value<T: serde::de::DeserializeOwned>(value: Value) -> T {
    match serde_json::from_value(value) {
        Ok(value) => value,
        Err(error) => panic!("failed to deserialize test value: {error}"),
    }
}
