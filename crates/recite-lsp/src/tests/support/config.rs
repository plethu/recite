use lsp_server::{Connection, Message};
use lsp_types::notification::{LogMessage, Notification as LspNotification};
use lsp_types::{InitializeResult, LogMessageParams, MessageType};
use recite_config::{ConfigError, LoadedUserConfig, UiLocale};
use recite_ui::{DEFAULT_RESOURCE, UiCatalog};
use serde_json::Value;

use super::Harness;
use crate::server::run_connection_with_user_config;

impl Harness {
    pub(in crate::tests) fn start_with_user_config(
        params: Value,
        loaded: Result<LoadedUserConfig, ConfigError>,
        locale: &str,
        resource: String,
    ) -> (Self, InitializeResult) {
        let (server_connection, client) = Connection::memory();
        let locale = locale.to_owned();
        let server = std::thread::spawn(move || {
            let default_catalog =
                UiCatalog::load(&UiLocale::default()).expect("test default catalog is complete");
            let requested = UiLocale::parse(&locale)
                .expect("test locale is valid")
                .resolve();
            run_connection_with_user_config(
                server_connection,
                loaded,
                default_catalog,
                move |resolved| {
                    assert_eq!(resolved.resolve(), requested);
                    UiCatalog::from_resources(
                        requested.clone(),
                        [
                            (
                                "en-US".parse().expect("English locale is valid"),
                                DEFAULT_RESOURCE.to_owned(),
                            ),
                            (requested.clone(), resource),
                        ],
                    )
                    .map_err(|error| error.to_string())
                },
            )
        });
        Self::finish_start(params, client, server)
    }

    pub(in crate::tests) fn start_with_user_config_and_resource(
        params: Value,
        loaded: Result<LoadedUserConfig, ConfigError>,
        locale: &str,
        resource: String,
    ) -> (Self, InitializeResult) {
        let (server_connection, client) = Connection::memory();
        let locale = locale.to_owned();
        let server = std::thread::spawn(move || {
            let requested = UiLocale::parse(&locale)
                .expect("test locale is valid")
                .resolve();
            let catalog = UiCatalog::from_resources(
                requested.clone(),
                [
                    (
                        UiLocale::parse("en-US").expect("locale").resolve(),
                        DEFAULT_RESOURCE.to_owned(),
                    ),
                    (requested, resource),
                ],
            )
            .expect("complete alternate catalog");
            run_connection_with_user_config(server_connection, loaded, catalog, |_| {
                Err("test catalog loader should not run".to_owned())
            })
        });
        Self::finish_start(params, client, server)
    }

    pub(in crate::tests) fn recv_log_message(&self) -> LogMessageParams {
        match self
            .client
            .receiver
            .recv_timeout(std::time::Duration::from_secs(1))
        {
            Ok(Message::Notification(notification)) => {
                assert_eq!(notification.method, LogMessage::METHOD);
                let params: LogMessageParams = super::from_value(notification.params);
                assert_eq!(params.typ, MessageType::WARNING);
                params
            }
            Ok(other) => panic!("expected log message notification, got {other:?}"),
            Err(error) => panic!("timed out or failed waiting for log message: {error}"),
        }
    }
}
