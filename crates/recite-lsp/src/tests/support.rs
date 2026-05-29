use std::thread::{self, JoinHandle};

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Exit, Initialized,
    Notification as LspNotification, PublishDiagnostics,
};
use lsp_types::request::{Request as LspRequest, Shutdown};
use lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    InitializeResult, PublishDiagnosticsParams, TextDocumentContentChangeEvent,
    TextDocumentIdentifier, TextDocumentItem, Uri, VersionedTextDocumentIdentifier,
};
use serde::Serialize;
use serde_json::{Value, json};

use crate::server::{ServerError, run_connection};

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
        match self.client.receiver.recv() {
            Ok(Message::Notification(notification)) => {
                assert_eq!(notification.method, PublishDiagnostics::METHOD);
                from_value(notification.params)
            }
            Ok(other) => panic!("expected diagnostics notification, got {other:?}"),
            Err(error) => panic!("failed to receive diagnostics: {error}"),
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
