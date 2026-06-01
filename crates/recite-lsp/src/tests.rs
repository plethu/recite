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
    fn did_save_without_project_state_is_an_explicit_no_op() {
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
    fn did_open_publishes_source_diagnostics_with_stable_shape() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/broken.recite");

        harness.did_open(uri.clone(), 7, "oops\n:ifx\n:: tavern\n");
        let published = harness.recv_publish_diagnostics();

        assert_eq!(published.uri, uri);
        assert_eq!(published.version, Some(7));
        assert_eq!(published.diagnostics.len(), 4);
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
                .map(|diagnostic| (
                    diagnostic.range.start.line,
                    diagnostic.range.start.character
                ))
                .collect::<Vec<_>>(),
            [(0, 0), (0, 0), (1, 0), (1, 0)]
        );

        harness.finish();
    }

    #[test]
    fn did_open_publishes_lowering_diagnostics() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/lowering.recite");
        let source = concat!(
            ":: tavern_arrival\n",
            "? ask_road\n",
            "  Ask about the road.\n",
            "    Wrong choice indent.\n",
            ":if knows_secret(player)\n",
            "  ! immediate play_sfx(ok)\n",
            "    ! immediate wrong_if_indent()\n",
            ":match thread_stage(thread)\n",
            "    :case ready\n",
            "      ! immediate play_sfx(ok)\n",
            "  :case tired\n",
            ":match mood(player)\n",
            "  :case calm\n",
            "    ! immediate play_sfx(ok)\n",
            "      ! immediate wrong_case_indent()\n",
        );

        harness.did_open(uri, 1, source);
        let published = harness.recv_publish_diagnostics();

        assert_eq!(published.diagnostics.len(), 4);
        assert_eq!(
            published
                .diagnostics
                .iter()
                .map(|diagnostic| match diagnostic.code.as_ref() {
                    Some(NumberOrString::String(code)) => code.as_str(),
                    _ => "<missing>",
                })
                .collect::<Vec<_>>(),
            [
                "RECITE_PARSE007",
                "RECITE_PARSE007",
                "RECITE_PARSE007",
                "RECITE_PARSE007"
            ]
        );

        harness.finish();
    }

    #[test]
    fn did_close_removes_state_and_clears_diagnostics() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/close.recite");

        harness.did_open(uri.clone(), 1, "oops\n:: tavern\n");
        assert!(!harness.recv_publish_diagnostics().diagnostics.is_empty());
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
        assert!(!harness.recv_publish_diagnostics().diagnostics.is_empty());

        harness.did_change(uri.clone(), 2, vec![full_change(":: tavern\n")]);
        let published = harness.recv_publish_diagnostics();
        assert_eq!(published.version, Some(2));
        assert!(published.diagnostics.is_empty());

        harness.finish();
    }

    #[test]
    fn stale_versions_do_not_publish_or_overwrite_newer_text() {
        let harness = Harness::start();
        let uri = uri("file:///workspace/dialogue/stale.recite");

        harness.did_open(uri.clone(), 3, ":: tavern\n");
        assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

        harness.did_change(uri.clone(), 2, vec![full_change("oops\n:: tavern\n")]);
        harness.assert_no_message();

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

mod project_indexes {
    use lsp_types::NumberOrString;
    use recite_core::{
        ContextualMetadataDomain, FlatMetadataDomain, MetadataContextSelector,
        MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema, RegistryDefinition,
    };
    use serde_json::json;
    use tempfile::TempDir;

    use crate::summary::{
        MetadataContextSelectorSummary, MetadataDomainKindSummary,
        MissingMetadataContextPolicySummary, ProvenanceSummary, SchemaSummary,
    };
    use crate::workspace::{LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};

    use super::support::{
        Harness, block_names, file_uri, full_change, harness_for_root, write_file,
    };

    #[test]
    fn saved_project_discovery_is_deterministically_sorted() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        write_file(temp.path(), "z.recite", ":: z\n");
        write_file(temp.path(), "a.recite", ":: a\n");
        write_file(temp.path(), "nested/m.recite", ":: nested\n");
        write_file(temp.path(), "target/ignored.recite", ":: ignored\n");
        write_file(temp.path(), ".hidden/ignored.recite", ":: ignored\n");

        let workspace = LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
        let paths = workspace
            .snapshot()
            .summaries()
            .iter()
            .map(|summary| {
                summary
                    .project_relative_path()
                    .unwrap_or("<none>")
                    .to_owned()
            })
            .collect::<Vec<_>>();

        assert_eq!(paths, ["a.recite", "nested/m.recite", "z.recite"]);
    }

    #[test]
    fn open_summary_overlays_saved_project_summary() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let source = temp.path().join("scene.recite");
        write_file(temp.path(), "scene.recite", ":: saved\n");

        let mut workspace =
            LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
        assert_eq!(block_names(&workspace), ["saved"]);

        workspace.open(file_uri(&source), 1, ":: live\n".to_owned());

        assert_eq!(block_names(&workspace), ["live"]);
        assert_eq!(workspace.snapshot().summaries()[0].version, Some(1));
    }

    #[test]
    fn did_save_rekeys_new_open_file_without_duplicate_summary() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let source = temp.path().join("draft.recite");
        let uri = file_uri(&source);
        let mut workspace =
            LspWorkspace::new(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

        workspace.open(uri.clone(), 1, ":: live\n".to_owned());
        assert_eq!(workspace.snapshot().summaries().len(), 1);
        assert!(
            workspace.snapshot().summaries()[0]
                .project_relative_path()
                .is_none()
        );

        write_file(temp.path(), "draft.recite", ":: saved\n");
        workspace.save(uri);

        assert_eq!(block_names(&workspace), ["live"]);
        let summaries = workspace.snapshot().summaries();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].version, Some(1));
        assert_eq!(summaries[0].project_relative_path(), Some("draft.recite"));
        assert!(summaries[0].saved_path().is_some());
    }

    #[test]
    fn did_save_refreshes_saved_summary_for_closed_files() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let source = temp.path().join("scene.recite");
        write_file(temp.path(), "scene.recite", ":: saved\n");
        let harness = harness_for_root(temp.path());

        write_file(temp.path(), "scene.recite", "oops\n:: saved\n");
        harness.did_save(file_uri(&source));
        let published = harness.recv_publish_diagnostics();

        assert!(
            published
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_ref()
                    == Some(&NumberOrString::String("RECITE_PARSE001".to_owned())))
        );

        harness.finish();
    }

    #[test]
    fn did_close_refreshes_saved_summary_before_falling_back() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let source = temp.path().join("scene.recite");
        write_file(temp.path(), "scene.recite", "oops\n:: saved\n");
        let harness = harness_for_root(temp.path());
        let uri = file_uri(&source);

        harness.did_open(uri.clone(), 1, ":: live\n");
        assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

        harness.did_close(uri);
        let published = harness.recv_publish_diagnostics();

        assert_eq!(published.version, None);
        assert!(
            published
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_ref()
                    == Some(&NumberOrString::String("RECITE_PARSE001".to_owned())))
        );

        harness.finish();
    }

    #[test]
    fn schema_load_failure_keeps_source_only_snapshot() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        write_file(temp.path(), "scene.recite", ":: source\n");
        write_file(temp.path(), "schema.json", r#"{"schema_version":"one"}"#);

        let workspace = LspWorkspace::new(
            WorkspaceConfig::for_roots(vec![temp.path().to_owned()])
                .with_schema_path(temp.path().join("schema.json")),
        );

        assert!(workspace.schema().summary().is_none());
        assert!(!workspace.schema().diagnostics().is_empty());
        assert_eq!(block_names(&workspace), ["source"]);
    }

    #[test]
    fn initialized_publishes_schema_load_diagnostics() {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let schema_path = temp.path().join("schema.json");
        write_file(temp.path(), "schema.json", r#"{"schema_version":"one"}"#);
        let root_uri = file_uri(temp.path());
        let schema_uri = file_uri(&schema_path);
        let (harness, _) = Harness::start_with_result(json!({
            "capabilities": {
                "general": {
                    "positionEncodings": ["utf-16"]
                }
            },
            "rootUri": root_uri.as_str(),
            "initializationOptions": {
                "schema": schema_path.display().to_string()
            }
        }));

        let published = harness.recv_publish_diagnostics();

        assert_eq!(published.uri, schema_uri);
        assert_eq!(published.version, None);
        assert_eq!(published.diagnostics.len(), 1);
        assert_eq!(
            published.diagnostics[0].code,
            Some(NumberOrString::String("RECITE_SCHEMA001".to_owned()))
        );

        harness.finish();
    }

    #[test]
    fn metadata_domain_schema_summary_preserves_available_provenance() {
        let mut schema = ProjectSchema::empty_v1();
        schema.registries.insert(
            "portrait".to_owned(),
            RegistryDefinition {
                values: ["smile".to_owned()].into_iter().collect(),
                origin: Some("data/portraits.toml".to_owned()),
            },
        );
        schema.metadata_domains.insert(
            "portrait_by_speaker".to_owned(),
            MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                selector: MetadataContextSelector::FieldSpeaker,
                values_by_context: [(
                    "hazel".to_owned(),
                    ["smile".to_owned()].into_iter().collect(),
                )]
                .into_iter()
                .collect(),
                missing_context: MissingMetadataContextPolicy::Diagnostic,
            }),
        );
        schema.metadata_domains.insert(
            "stage_by_tone".to_owned(),
            MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                selector: MetadataContextSelector::MetadataKey("tone".to_owned()),
                values_by_context: [(
                    "warm".to_owned(),
                    ["market".to_owned()].into_iter().collect(),
                )]
                .into_iter()
                .collect(),
                missing_context: MissingMetadataContextPolicy::Fallback {
                    domain: "tone".to_owned(),
                },
            }),
        );
        schema.metadata_domains.insert(
            "tone".to_owned(),
            MetadataDomainDefinition::Flat(FlatMetadataDomain {
                values: ["warm".to_owned()].into_iter().collect(),
            }),
        );

        let summary = SchemaSummary::from_schema(&schema);

        assert_eq!(
            summary.registries[0].provenance,
            ProvenanceSummary::Present {
                origin: "data/portraits.toml".to_owned()
            }
        );
        let speaker_domain = summary
            .metadata_domains
            .iter()
            .find(|domain| domain.name == "portrait_by_speaker")
            .expect("portrait_by_speaker metadata domain");
        assert_eq!(speaker_domain.provenance, ProvenanceSummary::Absent);
        match &speaker_domain.kind {
            MetadataDomainKindSummary::Contextual {
                selector,
                values_by_context,
                missing_context,
            } => {
                assert_eq!(selector, &MetadataContextSelectorSummary::FieldSpeaker);
                assert_eq!(values_by_context[0].context, "hazel");
                assert_eq!(values_by_context[0].values, ["smile"]);
                assert_eq!(
                    missing_context,
                    &MissingMetadataContextPolicySummary::Diagnostic
                );
            }
            other => panic!("unexpected metadata domain summary: {other:?}"),
        }

        let tone_domain = summary
            .metadata_domains
            .iter()
            .find(|domain| domain.name == "stage_by_tone")
            .expect("stage_by_tone metadata domain");
        match &tone_domain.kind {
            MetadataDomainKindSummary::Contextual {
                selector,
                values_by_context,
                missing_context,
            } => {
                assert_eq!(
                    selector,
                    &MetadataContextSelectorSummary::MetadataKey {
                        key: "tone".to_owned()
                    }
                );
                assert_eq!(values_by_context[0].context, "warm");
                assert_eq!(values_by_context[0].values, ["market"]);
                assert_eq!(
                    missing_context,
                    &MissingMetadataContextPolicySummary::Fallback {
                        domain: "tone".to_owned()
                    }
                );
            }
            other => panic!("unexpected metadata domain summary: {other:?}"),
        }
    }

    #[test]
    fn stale_change_does_not_bump_snapshot_generation() {
        let mut workspace = LspWorkspace::new(WorkspaceConfig::for_roots(Vec::new()));
        let uri = super::support::uri("file:///workspace/dialogue/stale-generation.recite");
        workspace.open(uri.clone(), 3, ":: live\n".to_owned());
        let generation = workspace.generation();

        match workspace.change(uri, 2, vec![full_change("oops\n:: stale\n")]) {
            WorkspaceChangeResult::Stale => {}
            other => panic!("expected stale change, got {other:?}"),
        }

        assert_eq!(workspace.generation(), generation);
        assert_eq!(workspace.snapshot().generation(), generation);
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
