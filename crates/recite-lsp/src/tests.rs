mod support;

mod lifecycle {
    use lsp_types::notification::{DidSaveTextDocument, Notification as LspNotification};
    use lsp_types::{
        ClientCapabilities, DidSaveTextDocumentParams, PositionEncodingKind,
        TextDocumentIdentifier, TextDocumentSyncCapability, TextDocumentSyncKind,
        TextDocumentSyncSaveOptions,
    };
    use serde_json::json;

    use super::support::{Harness, uri};

    #[test]
    fn initialize_advertises_full_sync_save_and_utf16() {
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

    #[test]
    fn initialize_defaults_to_utf16_when_client_lists_only_utf8() {
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

    #[test]
    fn did_save_is_an_explicit_no_op() {
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

    #[test]
    fn shutdown_request_and_exit_notification_terminate_loop() {
        let harness = Harness::start();
        harness.finish();
    }

    #[test]
    fn exit_before_shutdown_terminates_with_error() {
        let harness = Harness::start();

        match harness.exit_without_shutdown() {
            Err(crate::server::ServerError::ExitWithoutShutdown) => {}
            other => panic!("unexpected server result after early exit: {other:?}"),
        }
    }
}

mod diagnostics {
    use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};

    use super::support::{Harness, uri};

    #[test]
    fn did_open_publishes_parser_diagnostics_with_stable_shape() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/broken.recite");

        harness.did_open(uri.clone(), 7, "oops\n:ifx\n:: tavern\n");
        let published = harness.recv_publish_diagnostics();

        assert_eq!(published.uri, uri);
        assert_eq!(published.version, Some(7));
        assert_eq!(published.diagnostics.len(), 2);
        let diagnostic = &published.diagnostics[0];
        assert_eq!(
            diagnostic.code,
            Some(NumberOrString::String("RECITE_PARSE001".to_owned()))
        );
        assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::ERROR));
        assert_eq!(diagnostic.source.as_deref(), Some("recite"));
        assert_eq!(
            diagnostic.range,
            Range {
                start: Position {
                    line: 0,
                    character: 0
                },
                end: Position {
                    line: 0,
                    character: 0
                },
            }
        );
        assert_eq!(
            published
                .diagnostics
                .iter()
                .map(|diagnostic| diagnostic.range.start.line)
                .collect::<Vec<_>>(),
            [0, 1]
        );

        harness.finish();
    }

    #[test]
    fn did_close_removes_state_and_clears_diagnostics() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/close.recite");

        harness.did_open(uri.clone(), 1, "oops\n:: tavern\n");
        assert_eq!(harness.recv_publish_diagnostics().diagnostics.len(), 1);
        harness.did_close(uri.clone());
        let published = harness.recv_publish_diagnostics();
        assert_eq!(published.uri, uri);
        assert_eq!(published.version, None);
        assert!(published.diagnostics.is_empty());

        harness.finish();
    }
}

mod sync {
    use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    use super::support::{Harness, full_change, uri};

    #[test]
    fn full_change_replaces_and_clears_diagnostics() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/change.recite");

        harness.did_open(uri.clone(), 1, "oops\n:: tavern\n");
        assert_eq!(harness.recv_publish_diagnostics().diagnostics.len(), 1);

        harness.did_change(uri.clone(), 2, vec![full_change(":: tavern\n")]);
        let published = harness.recv_publish_diagnostics();
        assert_eq!(published.version, Some(2));
        assert!(published.diagnostics.is_empty());

        harness.finish();
    }

    #[test]
    fn stale_versions_do_not_overwrite_newer_text() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/stale.recite");

        harness.did_open(uri.clone(), 3, ":: tavern\n");
        assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

        harness.did_change(uri.clone(), 2, vec![full_change("oops\n:: tavern\n")]);
        let published = harness.recv_publish_diagnostics();
        assert_eq!(published.version, Some(3));
        assert!(published.diagnostics.is_empty());

        harness.finish();
    }

    #[test]
    fn non_full_or_malformed_changes_are_ignored() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/non-full.recite");

        harness.did_open(uri.clone(), 1, ":: tavern\n");
        assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

        harness.did_change(
            uri.clone(),
            2,
            vec![TextDocumentContentChangeEvent {
                range: Some(Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                }),
                range_length: None,
                text: "oops".to_owned(),
            }],
        );
        harness.did_change(
            uri.clone(),
            3,
            vec![full_change("oops"), full_change("\n:: tavern\n")],
        );
        harness.did_change(uri.clone(), 4, vec![full_change(":: tavern\n")]);
        let published = harness.recv_publish_diagnostics();
        assert_eq!(published.version, Some(4));
        assert!(published.diagnostics.is_empty());

        harness.finish();
    }

    #[test]
    fn change_for_unopened_document_is_ignored() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/unopened.recite");

        harness.did_change(uri, 1, vec![full_change("oops\n:: tavern\n")]);
        harness.assert_no_message();

        harness.finish();
    }
}

mod position {
    use lsp_types::{Position, Range};
    use recite_core::{SourcePosition, SourceSpan};

    #[test]
    fn crlf_and_non_bmp_text_use_utf16_ranges() {
        let range = crate::position::span_to_range(
            ":: tavern\r\n💬oops\r\n",
            &SourceSpan::new(
                "file:///workspace/dialogue/utf16.recite",
                source_position(2, 2),
                Some(source_position(2, 5)),
            ),
        );

        assert_eq!(
            range,
            Range {
                start: Position {
                    line: 1,
                    character: 2
                },
                end: Position {
                    line: 1,
                    character: 6
                },
            }
        );
    }

    fn source_position(line: u32, column: u32) -> SourcePosition {
        match SourcePosition::new(line, column) {
            Ok(position) => position,
            Err(error) => panic!("invalid source position {line}:{column}: {error}"),
        }
    }
}
