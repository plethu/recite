use lsp_types::{
    CompletionOptions, HoverProviderCapability, InitializeParams, InitializeResult,
    PositionEncodingKind, SaveOptions, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
};

pub(crate) fn initialize_result(params: &InitializeParams) -> InitializeResult {
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
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec!["(".to_owned(), "=".to_owned()]),
                ..CompletionOptions::default()
            }),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
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
