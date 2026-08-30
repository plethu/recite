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
    #[error("failed to load UI catalog: {0}")]
    UiCatalog(String),
    #[error("failed to publish LSP diagnostics: {0}")]
    Diagnostics(String),
    #[error("failed to initialize LSP authoring state: {0}")]
    Authoring(String),
}
