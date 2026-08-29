use lsp_server::Connection;
use lsp_types::InitializeParams;
use recite_ui::{UiCatalog, UiLocale};

use super::{Server, ServerError};
use crate::capabilities::initialize_result;
use crate::workspace::WorkspaceConfig;

pub fn run_stdio() -> Result<(), ServerError> {
    run_stdio_with_catalog(default_ui_catalog())
}

/// Run the language server with a caller-selected UI locale. Configuration
/// precedence belongs to #167; this explicit entry point is deterministic.
pub fn run_stdio_with_locale(locale: UiLocale) -> Result<(), ServerError> {
    let catalog =
        UiCatalog::load(&locale).map_err(|error| ServerError::UiCatalog(error.to_string()))?;
    run_stdio_with_catalog(catalog)
}

/// Run the language server with an injected shared UI catalog, useful for
/// embedding hosts and conformance tests.
pub fn run_stdio_with_catalog(catalog: UiCatalog) -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    run_connection_with_catalog(connection, catalog)?;
    io_threads.join()?;
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn run_connection(connection: Connection) -> Result<(), ServerError> {
    run_connection_with_catalog(connection, default_ui_catalog())
}

pub(crate) fn run_connection_with_catalog(
    connection: Connection,
    catalog: UiCatalog,
) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params = serde_json::from_value::<InitializeParams>(initialize_params)
        .unwrap_or_else(|_| InitializeParams::default());
    let initialize_result = initialize_result(&initialize_params);
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;
    let mut server = Server::new(
        connection,
        WorkspaceConfig::from_initialize_params(&initialize_params),
        catalog,
    );
    server.publish_schema_diagnostics()?;
    server.run()
}

#[allow(clippy::expect_used)]
fn default_ui_catalog() -> UiCatalog {
    UiCatalog::load(&UiLocale::default()).expect("embedded default UI catalog must load")
}
