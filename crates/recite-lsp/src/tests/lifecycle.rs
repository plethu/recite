use lsp_types::notification::{DidSaveTextDocument, Notification as LspNotification};
use lsp_types::{
    ClientCapabilities, DidSaveTextDocumentParams, PositionEncodingKind, TextDocumentIdentifier,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextDocumentSyncSaveOptions,
};
use serde_json::json;

use super::support::{Harness, uri};

pub(super) fn initialize_advertises_full_sync_save_and_utf16() {
    let (harness, result) = Harness::start_with_result(json!({
        "capabilities": ClientCapabilities::default()
    }));

    assert_eq!(
        result.capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );
    match result.capabilities.text_document_sync {
        Some(TextDocumentSyncCapability::Options(options)) => {
            assert_eq!(options.open_close, Some(true));
            assert_eq!(options.change, Some(TextDocumentSyncKind::FULL));
            assert_eq!(
                options.save,
                Some(TextDocumentSyncSaveOptions::SaveOptions(Default::default()))
            );
        }
        other => panic!("unexpected text document sync capability: {other:?}"),
    }

    harness.finish();
}

pub(super) fn initialize_defaults_to_utf16_when_client_lists_only_utf8() {
    let (harness, result) = Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-8"]
            }
        }
    }));

    assert_eq!(
        result.capabilities.position_encoding,
        Some(PositionEncodingKind::UTF16)
    );

    harness.finish();
}

pub(super) fn did_save_without_project_state_is_an_explicit_no_op() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/save.recite");

    harness.send_notification(
        DidSaveTextDocument::METHOD,
        DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text: None,
        },
    );
    harness.finish();
}

pub(super) fn shutdown_request_and_exit_notification_terminate_loop() {
    let harness = Harness::start();
    harness.finish();
}

pub(super) fn exit_before_shutdown_terminates_with_error() {
    let harness = Harness::start();

    match harness.exit_without_shutdown() {
        Err(crate::server::ServerError::ExitWithoutShutdown) => {}
        other => panic!("unexpected server result after early exit: {other:?}"),
    }
}
