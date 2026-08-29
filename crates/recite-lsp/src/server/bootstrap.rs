use lsp_server::{Connection, Request};
use lsp_types::InitializeParams;
use lsp_types::notification::Notification as _;
use lsp_types::request::Request as _;
use recite_config::{
    ConfigError, InvocationOverrides, LoadedUserConfig, load_user_config, resolve_user_config,
};
use recite_ui::{UiCatalog, UiLocale};

use super::{Server, ServerError};
use crate::capabilities::initialize_result;
use crate::workspace::WorkspaceConfig;

pub fn run_stdio() -> Result<(), ServerError> {
    let default_catalog = default_ui_catalog();
    let startup = startup_from_user_config(load_user_config(), default_catalog, |locale| {
        UiCatalog::load(locale).map_err(|error| error.to_string())
    })?;
    run_stdio_with_startup(startup)
}

/// Run the language server with a caller-selected UI locale.
///
/// This is an explicit embedding/test invocation override. Production startup
/// uses [`run_stdio`] and resolves the shared user configuration instead.
pub fn run_stdio_with_locale(locale: UiLocale) -> Result<(), ServerError> {
    let catalog =
        UiCatalog::load(&locale).map_err(|error| ServerError::UiCatalog(error.to_string()))?;
    run_stdio_with_catalog(catalog)
}

/// Run the language server with an injected shared UI catalog, useful for
/// embedding hosts and conformance tests.
pub fn run_stdio_with_catalog(catalog: UiCatalog) -> Result<(), ServerError> {
    run_stdio_with_startup(Startup::without_warning(catalog))
}

fn run_stdio_with_startup(startup: Startup) -> Result<(), ServerError> {
    let (connection, io_threads) = Connection::stdio();
    run_connection_with_startup(connection, startup)?;
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
    run_connection_with_startup(connection, Startup::without_warning(catalog))
}

#[allow(dead_code, reason = "used by in-crate lifecycle tests")]
pub(crate) fn run_connection_with_user_config(
    connection: Connection,
    loaded: Result<LoadedUserConfig, ConfigError>,
    default_catalog: UiCatalog,
    catalog_loader: impl FnOnce(&UiLocale) -> Result<UiCatalog, String>,
) -> Result<(), ServerError> {
    let startup = startup_from_user_config(loaded, default_catalog, catalog_loader)?;
    run_connection_with_startup(connection, startup)
}

fn run_connection_with_startup(
    connection: Connection,
    startup: Startup,
) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let initialize_params = serde_json::from_value::<InitializeParams>(initialize_params)
        .unwrap_or_else(|_| InitializeParams::default());
    let initialize_result = initialize_result(&initialize_params);
    connection.initialize_finish(initialize_id, serde_json::to_value(initialize_result)?)?;
    let mut server = Server::new(
        connection,
        WorkspaceConfig::from_initialize_params(&initialize_params),
        startup.catalog,
    );
    if initialize_params
        .capabilities
        .workspace
        .as_ref()
        .and_then(|workspace| workspace.did_change_watched_files.as_ref())
        .and_then(|capability| capability.dynamic_registration)
        .unwrap_or(false)
    {
        register_watched_files(&server)?;
    }
    if let Some(warning) = startup.warning {
        server.publish_startup_warning(warning)?;
    }
    server.publish_schema_diagnostics()?;
    server.run()
}

fn register_watched_files(server: &Server) -> Result<(), ServerError> {
    let options = lsp_types::DidChangeWatchedFilesRegistrationOptions {
        watchers: vec![lsp_types::FileSystemWatcher {
            glob_pattern: lsp_types::GlobPattern::String("**/*".to_owned()),
            kind: Some(
                lsp_types::WatchKind::Create
                    | lsp_types::WatchKind::Change
                    | lsp_types::WatchKind::Delete,
            ),
        }],
    };
    let registration = lsp_types::Registration {
        id: "recite-project-discovery".to_owned(),
        method: lsp_types::notification::DidChangeWatchedFiles::METHOD.to_owned(),
        register_options: Some(
            serde_json::to_value(options).map_err(ServerError::InitializeResult)?,
        ),
    };
    server.send(
        Request::new(
            lsp_server::RequestId::from(-167),
            lsp_types::request::RegisterCapability::METHOD.to_owned(),
            lsp_types::RegistrationParams {
                registrations: vec![registration],
            },
        )
        .into(),
    )
}

struct Startup {
    catalog: UiCatalog,
    warning: Option<String>,
}

impl Startup {
    fn without_warning(catalog: UiCatalog) -> Self {
        Self {
            catalog,
            warning: None,
        }
    }
}

fn startup_from_user_config(
    loaded: Result<LoadedUserConfig, ConfigError>,
    default_catalog: UiCatalog,
    catalog_loader: impl FnOnce(&UiLocale) -> Result<UiCatalog, String>,
) -> Result<Startup, ServerError> {
    match loaded {
        Ok(loaded) => {
            let resolved = resolve_user_config(&loaded, &InvocationOverrides::new());
            let ui = resolved.ui();
            let locale = ui.locale().value();
            let catalog = catalog_loader(locale).map_err(ServerError::UiCatalog)?;
            Ok(Startup::without_warning(catalog))
        }
        Err(error) => Ok(Startup {
            warning: Some(config_warning(&default_catalog, &error)),
            catalog: default_catalog,
        }),
    }
}

fn config_warning(catalog: &UiCatalog, error: &ConfigError) -> String {
    catalog.format_pairs(
        recite_ui::MsgId::LspWarningUiConfig,
        [
            ("code", error.diagnostic().code().as_str().to_owned()),
            ("detail", error.to_string()),
        ],
    )
}

fn default_ui_catalog() -> UiCatalog {
    #[expect(
        clippy::expect_used,
        reason = "the embedded default UI catalog is validated by the UI contract gate"
    )]
    {
        UiCatalog::load(&UiLocale::default()).expect("embedded default UI catalog must load")
    }
}
