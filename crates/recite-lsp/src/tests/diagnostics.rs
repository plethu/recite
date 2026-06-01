use lsp_types::{DiagnosticSeverity, NumberOrString, Position, Range};

use super::support::{Harness, uri};

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
