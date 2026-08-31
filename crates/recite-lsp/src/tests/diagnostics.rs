use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation,
    DiagnosticPresentationId, DiagnosticRelatedPresentation, SourcePosition, SourceSpan,
    contract_for,
};
use serde_json::json;
use tempfile::TempDir;

use super::support::{Harness, file_uri, full_change, test_workspace, uri, write_file};
use crate::diagnostics::publish_diagnostics;
use crate::workspace::WorkspaceConfig;

mod fixtures;
mod schema;

pub(super) fn did_open_publishes_source_diagnostics_with_stable_shape() {
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

pub(super) fn did_open_publishes_lowering_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/lowering.recite");
    let source = concat!(
        ":: tavern_arrival\n",
        "? ask_road@a5b41169900e68f23ea0\n",
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

pub(super) fn did_open_publishes_schema_less_semantic_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/semantic.recite");

    harness.did_open(
        uri.clone(),
        1,
        concat!(
            ":: start default\n",
            ">\n",
            "  Missing line id.\n",
            "> repeated@83709c28414d0ce4659c\n",
            "  First repeated line id.\n",
            "> prompt@b6b804baf5b0ea3ec34a\n",
            "  Prompt.\n",
            "  ? repeated_choice@88d47ec76de1bbce527a\n",
            "    First repeated choice.\n",
            "    -> missing_block\n",
            "  ? other_choice_label@88d47ec76de1bbce527a\n",
            "    Second repeated choice.\n",
            "    -> END\n",
            "> other_line_label@83709c28414d0ce4659c\n",
            "  Second repeated line id.\n",
        ),
    );
    let published = harness.recv_publish_diagnostics();

    assert_eq!(published.uri, uri.clone());
    assert_eq!(published.version, Some(1));
    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        [
            "RECITE_ID001",
            "RECITE_VALIDATE007",
            "RECITE_ID004",
            "RECITE_ID003"
        ]
    );
    assert_eq!(published.diagnostics[0].range.start, Position::new(1, 0));
    assert_eq!(published.diagnostics[1].range.start, Position::new(9, 4));
    let related = published.diagnostics[2]
        .related_information
        .as_ref()
        .expect("duplicate choice ID has a related source span");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].message, "first localisable ID is here");

    harness.did_change(
        uri,
        2,
        vec![full_change(concat!(
            ":: start default\n",
            "> fixed@64cd2c5a62499a4e9bb4\n",
            "  Fixed.\n",
        ))],
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.version, Some(2));
    assert!(published.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn shared_language_pressure_fixture_publishes_no_diagnostics() {
    fixtures::shared_language_pressure_fixture_publishes_no_diagnostics();
}

pub(super) fn shared_language_pressure_fixture_projects_marker_diagnostics() {
    fixtures::shared_language_pressure_fixture_projects_marker_diagnostics();
}

pub(super) fn did_open_publishes_schema_backed_semantic_diagnostics() {
    schema::did_open_publishes_schema_backed_semantic_diagnostics();
}

fn diagnostic_codes(diagnostics: &[lsp_types::Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .map(|diagnostic| match diagnostic.code.as_ref() {
            Some(NumberOrString::String(code)) => code.as_str(),
            _ => "<missing>",
        })
        .collect()
}

pub(super) fn related_spans_resolve_project_files_and_target_text() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first_path = temp.path().join("dialogue/first.recite");
    write_file(
        temp.path(),
        "dialogue/first.recite",
        ":: first\n> shared@83709c28414d0ce4659c\n  😀First.\n",
    );
    let first_uri = file_uri(&first_path);
    let second_uri = file_uri(&temp.path().join("dialogue/second.recite"));
    let root_uri = file_uri(temp.path());
    let harness = Harness::start_with_result(json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": root_uri.as_str(),
    }))
    .0;

    harness.did_open(
        second_uri.clone(),
        1,
        ":: second\n> shared@83709c28414d0ce4659c\n  Second.\n",
    );
    let published = harness.recv_publish_diagnostics();
    let duplicate = published
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.code == Some(NumberOrString::String("RECITE_ID003".to_owned()))
        })
        .expect("duplicate line ID diagnostic");
    let related = duplicate
        .related_information
        .as_ref()
        .expect("duplicate line ID has related source");
    assert_eq!(related.len(), 1);
    assert_eq!(related[0].location.uri, first_uri);
    assert_eq!(related[0].location.range.start, Position::new(1, 0));
    assert_eq!(related[0].location.range.end, Position::new(1, 0));

    let workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    let primary_span = SourceSpan::point(
        "dialogue/second.recite",
        SourcePosition::new(1, 1).expect("valid test position"),
    );
    let related_span = SourceSpan::new(
        "dialogue/first.recite",
        SourcePosition::new(3, 3).expect("valid test position"),
        Some(SourcePosition::new(3, 8).expect("valid test position")),
    );
    let code = DiagnosticCode::new_static("RECITE_ID003");
    let contract = contract_for(
        &code,
        &DiagnosticPresentationId::new_static("diagnostic-id-003"),
    )
    .expect("duplicate ID contract");
    let diagnostic = Diagnostic::error_from_contract(
        contract,
        "compatibility message",
        primary_span,
        [("id", DiagnosticArgumentValue::String("shared".to_owned()))],
    )
    .expect("duplicate ID arguments match contract")
    .with_related_presentations([DiagnosticRelatedPresentation::new(
        related_span,
        DiagnosticPresentation::new(DiagnosticPresentationId::new_static(
            "diagnostic-id-003-related",
        )),
    )]);
    let published = publish_diagnostics(
        second_uri.clone(),
        ":: second\n> shared@83709c28414d0ce4659c\n  Second.\n",
        Some(1),
        &[diagnostic],
        &workspace.ui_catalog,
        &workspace.diagnostic_sources_for_uri(&second_uri),
    )
    .expect("recordable diagnostic");
    let related = published.diagnostics[0]
        .related_information
        .as_ref()
        .expect("synthetic diagnostic has related source");
    assert_eq!(related[0].location.uri, first_uri);
    assert_eq!(related[0].location.range.start, Position::new(2, 2));
    assert_eq!(related[0].location.range.end, Position::new(2, 9));

    harness.finish();
}

pub(super) fn did_save_publishes_schema_backed_diagnostics_for_closed_project_files() {
    schema::did_save_publishes_schema_backed_diagnostics_for_closed_project_files();
}

pub(super) fn did_save_schema_reloads_and_republishes_source_diagnostics() {
    schema::did_save_schema_reloads_and_republishes_source_diagnostics();
}

pub(super) fn did_save_schema_reloads_from_non_canonical_schema_uri() {
    schema::did_save_schema_reloads_from_non_canonical_schema_uri();
}

pub(super) fn did_save_keeps_unsaved_schema_overlay() {
    schema::did_save_keeps_unsaved_schema_overlay();
}

pub(super) fn watched_schema_refresh_keeps_unsaved_schema_overlay() {
    schema::watched_schema_refresh_keeps_unsaved_schema_overlay();
}

pub(super) fn valid_schema_overlay_clears_diagnostics_with_new_version() {
    schema::valid_schema_overlay_clears_diagnostics_with_new_version();
}

pub(super) fn did_close_schema_alias_clears_exact_uri() {
    schema::did_close_schema_alias_clears_exact_uri();
}

pub(super) fn did_close_removes_state_and_clears_diagnostics() {
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
