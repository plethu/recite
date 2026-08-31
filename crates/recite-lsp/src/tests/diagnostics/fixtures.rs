use lsp_types::{NumberOrString, Position, Range};

use super::super::support::{Harness, uri};

pub(super) fn shared_language_pressure_fixture_publishes_no_diagnostics() {
    let harness = Harness::start();
    let source_uri = uri("file:///workspace/fixtures/recite/valid/language_pressure.recite");
    let source = include_str!("../../../../../fixtures/recite/valid/language_pressure.recite");

    harness.did_open(source_uri, 1, source);
    let published = harness.recv_publish_diagnostics();

    assert!(published.diagnostics.is_empty(), "{published:?}");
    harness.finish();
}

pub(super) fn shared_language_pressure_fixture_projects_marker_diagnostics() {
    let harness = Harness::start();
    let source_uri =
        uri("file:///workspace/fixtures/recite/invalid/parser_marker_leading_prose.recite");
    let source =
        include_str!("../../../../../fixtures/recite/invalid/parser_marker_leading_prose.recite");

    harness.did_open(source_uri, 1, source);
    let published = harness.recv_publish_diagnostics();

    assert_eq!(
        diagnostic_codes(&published.diagnostics),
        ["RECITE_PARSE011", "RECITE_PARSE013"]
    );
    assert_eq!(
        published.diagnostics[0].range,
        Range {
            start: Position::new(2, 11),
            end: Position::new(2, 13),
        }
    );
    assert_eq!(
        published.diagnostics[1].range,
        Range {
            start: Position::new(3, 11),
            end: Position::new(3, 13),
        }
    );

    harness.finish();
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
