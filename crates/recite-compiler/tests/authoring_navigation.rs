#![cfg(test)]

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, NavigationResult, QueryResult, SavedDocument,
};
use recite_core::{DocumentKey, SourcePosition};

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test key is valid")
}

#[test]
fn navigation_scopes_unqualified_and_qualified_targets_deterministically() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            recite_compiler::SnapshotGeneration::initial(),
            [
                SavedDocument::new(
                    key("main.recite"),
                    ":: main\n-> finish\n-> a.recite::finish\n:: finish\n:: finish\n",
                ),
                SavedDocument::new(key("a.recite"), ":: finish\n"),
                SavedDocument::new(key("b.recite"), ":: finish\n"),
            ],
            [],
        ))
        .expect("navigation fixture accepted");

    let local = SourcePosition::new(2, 4).expect("valid source position");
    let QueryResult::Ready(NavigationResult::Ambiguous(locations)) =
        kernel.snapshot().navigate(&key("main.recite"), local)
    else {
        panic!("unqualified navigation is ambiguous in the current document");
    };
    assert_eq!(locations.len(), 2);
    assert!(
        locations
            .iter()
            .all(|location| location.document().as_str() == "main.recite")
    );

    let qualified = SourcePosition::new(3, 16).expect("qualified source position");
    let QueryResult::Ready(NavigationResult::Unique(location)) =
        kernel.snapshot().navigate(&key("main.recite"), qualified)
    else {
        panic!("qualified target resolves uniquely");
    };
    assert_eq!(location.document().as_str(), "a.recite");

    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(
                key("main.recite"),
                ":: main\n-> absent\n",
            )],
            [],
        ))
        .expect("missing target replacement accepted");
    let QueryResult::Ready(NavigationResult::Missing) =
        kernel.snapshot().navigate(&key("main.recite"), local)
    else {
        panic!("missing navigation is explicit");
    };
}

#[test]
fn missing_ids_keep_typed_identity_and_exact_insertion_points_across_crlf_utf8() {
    let source = ":: start\r\n> \r\n  💬\r\n? \r\n  Pick this\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            recite_compiler::SnapshotGeneration::initial(),
            [SavedDocument::new(key("unicode.recite"), source)],
            [],
        ))
        .expect("unicode fixture accepted with recovery");
    let document = kernel
        .snapshot()
        .document(&key("unicode.recite"))
        .expect("unicode document exists");
    assert_eq!(document.metadata().byte_len(), source.len());
    assert_eq!(document.summary().stable_ids().len(), 2);
    for stable in document.summary().stable_ids() {
        assert!(matches!(stable.source_id(), recite_core::SourceId::Missing));
        assert!(stable.source_id_span().is_none());
    }
    assert_eq!(
        document.summary().stable_ids()[0]
            .insertion_span()
            .expect("parser insertion span")
            .start,
        recite_core::SourcePosition::new(2, 3).expect("valid insertion position")
    );
    assert_eq!(
        document.summary().stable_ids()[1]
            .insertion_span()
            .expect("parser insertion span")
            .start,
        recite_core::SourcePosition::new(4, 3).expect("valid insertion position")
    );
}
