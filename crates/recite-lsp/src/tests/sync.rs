use lsp_types::{Position, Range, TextDocumentContentChangeEvent};

use super::support::{Harness, full_change, uri};

pub(super) fn full_change_replaces_and_clears_diagnostics() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/change.recite");

    harness.did_open(uri.clone(), 1, "oops\n:: tavern\n");
    assert!(!harness.recv_publish_diagnostics().diagnostics.is_empty());

    harness.did_change(
        uri.clone(),
        2,
        vec![full_change(
            ":: tavern default\n> intro@1f6bec0fb5fdbe141952\n  Hello.\n",
        )],
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.version, Some(2));
    assert!(published.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn stale_versions_do_not_publish_or_overwrite_newer_text() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/stale.recite");

    harness.did_open(
        uri.clone(),
        3,
        ":: tavern default\n> intro@b582eea0d14bd11d5bad\n  Hello.\n",
    );
    assert!(harness.recv_publish_diagnostics().diagnostics.is_empty());

    harness.did_change(uri.clone(), 2, vec![full_change("oops\n:: tavern\n")]);
    harness.assert_no_message();

    harness.finish();
}

pub(super) fn non_full_or_malformed_changes_are_ignored() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/non-full.recite");

    harness.did_open(
        uri.clone(),
        1,
        ":: tavern default\n> intro@fbe31c8fe0289a3d5d4d\n  Hello.\n",
    );
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
    harness.did_change(
        uri.clone(),
        4,
        vec![full_change(
            ":: tavern default\n> intro@d2547260577d7b3d4ead\n  Hello.\n",
        )],
    );
    let published = harness.recv_publish_diagnostics();
    assert_eq!(published.version, Some(4));
    assert!(published.diagnostics.is_empty());

    harness.finish();
}

pub(super) fn change_for_unopened_document_is_ignored() {
    let harness = Harness::start();
    let uri = uri("file:///workspace/dialogue/unopened.recite");

    harness.did_change(uri, 1, vec![full_change("oops\n:: tavern\n")]);
    harness.assert_no_message();

    harness.finish();
}
