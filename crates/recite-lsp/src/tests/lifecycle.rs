use lsp_types::notification::{DidSaveTextDocument, Notification as LspNotification};
use lsp_types::request::{RegisterCapability, Request as LspRequest};
use lsp_types::{
    ClientCapabilities, DidSaveTextDocumentParams, NumberOrString, Position, PositionEncodingKind,
    TextDocumentIdentifier, TextDocumentSyncCapability, TextDocumentSyncKind,
    TextDocumentSyncSaveOptions,
};
use recite_config::{
    ConfigError, Platform, PlatformRoots, PlayConfig, UiConfig, UiLocale, UserConfig,
    load_user_config_from,
};
use recite_ui::DEFAULT_RESOURCE;
use serde_json::json;
use std::path::PathBuf;
use tempfile::tempdir;

use super::support::{Harness, uri};

pub(super) fn dynamic_watched_files_register_after_initialized() {
    let (mut harness, _) = Harness::start_with_result(json!({
        "capabilities": {
            "workspace": {
                "didChangeWatchedFiles": { "dynamicRegistration": true }
            }
        }
    }));
    let registration_request = harness.recv_request();
    assert_eq!(registration_request.method, RegisterCapability::METHOD);
    let registration: lsp_types::RegistrationParams =
        serde_json::from_value(registration_request.params)
            .unwrap_or_else(|error| panic!("dynamic registration params: {error}"));
    assert_eq!(registration.registrations.len(), 1);
    let registration = &registration.registrations[0];
    assert_eq!(registration.id, "recite-project-discovery");
    assert_eq!(registration.method, "workspace/didChangeWatchedFiles");
    let options = registration
        .register_options
        .as_ref()
        .unwrap_or_else(|| panic!("watched-files registration options are missing"));
    assert_eq!(options["watchers"][0]["globPattern"], "**/*");
    assert_eq!(options["watchers"][0]["kind"], 7);
    harness.reply_ok(registration_request.id);

    harness.send_initialized();
    assert!(
        harness
            .completion(
                uri("file:///workspace/dialogue/test.recite"),
                Position::new(0, 0)
            )
            .is_none()
    );
    harness.finish();
}

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

pub(super) fn valid_ui_config_changes_presentation_only() {
    let loaded = UserConfig {
        ui: UiConfig {
            locale: UiLocale::parse("fr-FR").expect("locale"),
            ..UiConfig::default()
        },
        play: PlayConfig::default(),
        config_version: recite_config::CONFIG_VERSION,
    };
    let localized = DEFAULT_RESOURCE.replace(
        "diagnostic-parse-001 = expected a Recite statement header or indented prose",
        "diagnostic-parse-001 = diagnostic localisé",
    );
    let (harness, result) = super::support::Harness::start_with_user_config(
        json!({"capabilities": ClientCapabilities::default()}),
        Ok(recite_config::LoadedUserConfig::from_explicit(loaded)),
        "fr-FR",
        localized,
    );

    assert!(result.capabilities.completion_provider.is_some());
    harness.assert_no_message();

    let uri = super::support::uri("file:///workspace/dialogue/configured.recite");
    harness.did_open(uri, 1, "oops\n");
    let published = harness.recv_publish_diagnostics();
    let diagnostic = published
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == Some(NumberOrString::String("RECITE_PARSE001".to_owned()))
        })
        .expect("parse diagnostic");
    assert_eq!(diagnostic.message, "diagnostic localisé");
    assert_eq!(diagnostic.range.start, Position::new(0, 0));
    assert_eq!(diagnostic.range.end, Position::new(0, 0));

    harness.finish();
}

pub(super) fn absent_platform_default_uses_defaults_without_warning() {
    let loaded = load_user_config_from(Platform::Linux, &PlatformRoots::new(), None)
        .expect("missing platform root uses defaults");
    let (harness, result) = super::support::Harness::start_with_user_config(
        json!({"capabilities": ClientCapabilities::default()}),
        Ok(loaded),
        "en-US",
        DEFAULT_RESOURCE.to_owned(),
    );

    assert!(result.capabilities.completion_provider.is_some());
    harness.assert_no_message();
    harness.finish();
}

pub(super) fn malformed_user_config_warns_without_blocking_initialize() {
    let temp = tempdir().expect("temporary directory");
    let schema_path = temp.path().join("missing-schema.json");
    let error = ConfigError::Malformed {
        path: PathBuf::from("/synthetic/recite-config.toml"),
        message: "invalid TOML".to_owned(),
    };
    let (harness, result) = super::support::Harness::start_with_user_config(
        json!({
            "capabilities": ClientCapabilities::default(),
            "initializationOptions": {"schema": schema_path.display().to_string()}
        }),
        Err(error),
        "fr-FR",
        DEFAULT_RESOURCE.to_owned(),
    );

    assert!(result.capabilities.hover_provider.is_some());
    let warning = harness.recv_log_message();
    assert!(warning.message.contains("RECITE_CONFIG005"));
    assert!(warning.message.contains("invalid TOML"));
    let schema_diagnostics = harness.recv_publish_diagnostics();
    assert_eq!(schema_diagnostics.diagnostics.len(), 1);

    harness.finish();
}

pub(super) fn explicit_missing_user_config_warns_with_stable_code() {
    let error = ConfigError::MissingExplicit {
        path: PathBuf::from("/synthetic/missing-recite-config.toml"),
    };
    let (harness, _) = super::support::Harness::start_with_user_config(
        json!({"capabilities": ClientCapabilities::default()}),
        Err(error),
        "fr-FR",
        DEFAULT_RESOURCE.to_owned(),
    );

    let warning = harness.recv_log_message();
    assert_eq!(warning.typ, lsp_types::MessageType::WARNING);
    assert!(warning.message.contains("RECITE_CONFIG003"));
    assert!(warning.message.contains("missing-recite-config.toml"));
    harness.assert_no_message();
    harness.finish();
}

pub(super) fn translated_config_warning_uses_exact_code_and_detail_once() {
    let localized = DEFAULT_RESOURCE.replace(
        "lsp-warning-ui-config = UI configuration could not be loaded (code {$code}): {$detail}; using embedded en-US UI text.",
        "lsp-warning-ui-config = localized config warning [{$code}] {$detail}",
    );
    let error = ConfigError::Malformed {
        path: PathBuf::from("/synthetic/recite-config.toml"),
        message: "invalid TOML".to_owned(),
    };
    let (harness, _) = super::support::Harness::start_with_user_config_and_resource(
        json!({"capabilities": ClientCapabilities::default()}),
        Err(error),
        "fr-FR",
        localized,
    );

    let warning = harness.recv_log_message();
    assert_eq!(
        warning.message,
        "localized config warning [RECITE_CONFIG005] could not parse user config /synthetic/recite-config.toml: invalid TOML"
    );
    harness.assert_no_message();
    harness.finish();
}
