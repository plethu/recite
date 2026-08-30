#![cfg(test)]

use recite_compiler::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentLayer, DocumentVersion,
    OpenDocument, QueryResult, SavedDocument, SnapshotGeneration,
};
use recite_core::{DocumentKey, SourceId, SourcePosition};

fn key(value: &str) -> DocumentKey {
    match DocumentKey::new(value) {
        Ok(key) => key,
        Err(error) => panic!("test key is valid: {error}"),
    }
}

fn request(
    generation: SnapshotGeneration,
    saved: impl IntoIterator<Item = SavedDocument>,
    open: impl IntoIterator<Item = OpenDocument>,
) -> AuthoringRequest {
    AuthoringRequest::new(generation, saved, open)
}

fn saved(name: &str, text: &str) -> SavedDocument {
    SavedDocument::new(key(name), text)
}

fn open(name: &str, version: i64, text: &str) -> OpenDocument {
    OpenDocument::new(key(name), DocumentVersion::new(version), text)
}

#[test]
fn saved_and_overlay_inputs_are_sorted_and_overlay_wins() {
    let mut kernel = AuthoringKernel::new();
    let _initial_delta = kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("z.recite", ":: z\n"), saved("a.recite", ":: a\n")],
            [
                open("z.recite", -2, ":: overlay\n"),
                open("open.recite", i64::MAX, ":: open\n"),
            ],
        ))
        .expect("initial request accepted");

    let documents = kernel.snapshot().documents();
    assert_eq!(documents.len(), 3);
    assert_eq!(documents[0].key().as_str(), "a.recite");
    assert_eq!(documents[1].key().as_str(), "open.recite");
    assert_eq!(documents[2].key().as_str(), "z.recite");
    assert_eq!(documents[2].layer(), DocumentLayer::Open);
    assert_eq!(
        documents[2].version().map(DocumentVersion::as_i64),
        Some(-2)
    );
    assert_eq!(documents[2].metadata().byte_len(), ":: overlay\n".len());

    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(
            generation,
            [saved("z.recite", ":: saved\n"), saved("a.recite", ":: a\n")],
            [],
        ))
        .expect("closing overlays falls back to saved inputs");
    assert_eq!(
        kernel
            .snapshot()
            .document(&key("z.recite"))
            .map(|document| document.layer()),
        Some(DocumentLayer::Saved)
    );
    assert!(kernel.snapshot().document(&key("open.recite")).is_none());
}

#[test]
fn duplicate_layers_and_versions_are_typed_and_transactional() {
    let mut kernel = AuthoringKernel::new();
    let initial = request(
        SnapshotGeneration::initial(),
        [saved("a.recite", ":: a\n")],
        [open("a.recite", 1, ":: open\n")],
    );
    kernel.apply(initial).expect("initial request accepted");
    let before = kernel.snapshot().clone();

    let error = kernel
        .apply(request(
            before.generation(),
            [
                saved("a.recite", ":: a\n"),
                saved("a.recite", ":: duplicate\n"),
            ],
            [],
        ))
        .expect_err("duplicate saved documents fail");
    assert!(matches!(
        error,
        AuthoringError::DuplicateSavedDocument { .. }
    ));
    assert_eq!(kernel.snapshot(), &before);

    let error = kernel
        .apply(request(
            before.generation(),
            [saved("a.recite", ":: a\n")],
            [open("a.recite", 1, ":: changed\n")],
        ))
        .expect_err("same-version changed text fails");
    assert!(matches!(
        error,
        AuthoringError::OverlayVersionConflict { .. }
    ));
    assert_eq!(kernel.snapshot(), &before);

    let error = kernel
        .apply(request(
            before.generation(),
            [saved("a.recite", ":: a\n")],
            [open("a.recite", 0, ":: old\n")],
        ))
        .expect_err("decreasing version fails");
    assert!(matches!(error, AuthoringError::StaleOverlayVersion { .. }));
    assert_eq!(kernel.snapshot(), &before);
}

#[test]
fn equal_overlay_input_is_a_no_op_and_close_resets_lifecycle() {
    let mut kernel = AuthoringKernel::new();
    let _initial_delta = kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [],
            [open("a.recite", i32::MIN as i64, ":: a\n")],
        ))
        .expect("initial request accepted");
    let snapshot = kernel.snapshot().clone();
    let no_op_delta = kernel
        .apply(request(
            snapshot.generation(),
            [],
            [open("a.recite", i32::MIN as i64, ":: a\n")],
        ))
        .expect("identical request is accepted as a no-op");
    assert_eq!(kernel.snapshot(), &snapshot);
    assert!(no_op_delta.is_empty());
    assert_eq!(no_op_delta.previous_generation(), snapshot.generation());
    assert_eq!(no_op_delta.generation(), snapshot.generation());

    let _close_delta = kernel
        .apply(request(snapshot.generation(), [], []))
        .expect("close accepted");
    let generation = kernel.snapshot().generation();
    let _reopen_delta = kernel
        .apply(request(
            generation,
            [],
            [open("a.recite", i32::MAX as i64, ":: a\n")],
        ))
        .expect("reopen accepts any first version");
    assert_eq!(
        kernel.snapshot().documents()[0]
            .version()
            .map(DocumentVersion::as_i64),
        Some(i32::MAX as i64)
    );
}

#[test]
fn generation_mismatch_is_transactional() {
    let mut kernel = AuthoringKernel::new();
    let error = kernel
        .apply(request(
            SnapshotGeneration::new(42),
            [saved("a.recite", ":: a\n")],
            [],
        ))
        .expect_err("wrong expected generation fails");
    assert!(matches!(error, AuthoringError::GenerationMismatch { .. }));
    assert_eq!(
        kernel.snapshot().generation(),
        SnapshotGeneration::initial()
    );
    assert!(kernel.snapshot().documents().is_empty());
}

#[test]
fn malformed_document_keeps_parser_diagnostics_without_poisoning_clean_validation() {
    let mut kernel = AuthoringKernel::new();
    let _initial_delta = kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [
                saved(
                    "clean.recite",
                    concat!(
                        ":: start default\n",
                        ">\n",
                        "  A line without an ID.\n",
                        "-> malformed.recite::missing\n",
                        "! immediate missing_effect()\n",
                    ),
                ),
                saved("malformed.recite", "oops\n"),
            ],
            [],
        ))
        .expect("recoverable source request accepted");

    let clean = kernel
        .snapshot()
        .document(&key("clean.recite"))
        .expect("clean document is present");
    let malformed = kernel
        .snapshot()
        .document(&key("malformed.recite"))
        .expect("malformed document is present");
    assert!(
        malformed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str().starts_with("RECITE_PARSE"))
    );
    assert!(!malformed.participation().ast_structure().is_complete());
    assert!(!malformed.participation().block_definitions().is_complete());
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_ID001")
    );
    assert!(
        clean
            .diagnostics()
            .iter()
            .all(|diagnostic| diagnostic.code.as_str() != "RECITE_VALIDATE007")
    );

    let generation = kernel.snapshot().generation();
    let _initial_delta = kernel
        .apply(request(
            generation,
            [
                saved(
                    "clean.recite",
                    concat!(
                        ":: start default\n",
                        "> line@11111111111111111111\n",
                        "  A line with an ID.\n",
                        "-> malformed.recite::missing\n",
                    ),
                ),
                saved("malformed.recite", ":: known\n"),
            ],
            [],
        ))
        .expect("complete replacement accepted");
    let clean = kernel
        .snapshot()
        .document(&key("clean.recite"))
        .expect("clean document remains present");
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_VALIDATE007")
    );
}

#[test]
fn delta_contains_sorted_changed_and_removed_metadata() {
    let mut kernel = AuthoringKernel::new();
    let initial_delta = kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("z.recite", ":: z\n"), saved("a.recite", ":: a\n")],
            [],
        ))
        .expect("initial request accepted");
    assert_eq!(
        initial_delta
            .changed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite", "z.recite"]
    );

    let generation = kernel.snapshot().generation();
    let delta = kernel
        .apply(request(generation, [saved("z.recite", ":: changed\n")], []))
        .expect("replacement accepted");
    assert_eq!(
        delta
            .removed()
            .iter()
            .map(|change| change.key().as_str())
            .collect::<Vec<_>>(),
        ["a.recite"]
    );
    assert_eq!(
        delta.changed()[0]
            .previous()
            .map(|metadata| metadata.byte_len()),
        Some(":: z\n".len())
    );
    assert_eq!(
        delta.changed()[0]
            .current()
            .map(|metadata| metadata.byte_len()),
        Some(":: changed\n".len())
    );
}

#[test]
fn navigation_reports_ambiguous_and_missing_targets_deterministically() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [
                saved(
                    "main.recite",
                    ":: main\n-> finish\n-> a.recite::finish\n:: finish\n:: finish\n",
                ),
                saved("a.recite", ":: finish\n"),
                saved("b.recite", ":: finish\n"),
            ],
            [],
        ))
        .expect("navigation fixture accepted");
    let position = SourcePosition::new(2, 4).expect("valid source position");
    let QueryResult::Ready(recited) = kernel.snapshot().navigate(&key("main.recite"), position)
    else {
        panic!("navigation is ready");
    };
    let recite_compiler::NavigationResult::Ambiguous(locations) = recited else {
        panic!("two declarations are ambiguous");
    };
    assert_eq!(
        locations
            .iter()
            .map(|location| location.document().as_str())
            .collect::<Vec<_>>(),
        ["main.recite", "main.recite"]
    );

    let qualified = SourcePosition::new(3, 16).expect("qualified source position");
    let QueryResult::Ready(recited) = kernel.snapshot().navigate(&key("main.recite"), qualified)
    else {
        panic!("qualified navigation is ready");
    };
    let recite_compiler::NavigationResult::Unique(location) = recited else {
        panic!("qualified target resolves uniquely");
    };
    assert_eq!(location.document().as_str(), "a.recite");

    let generation = kernel.snapshot().generation();
    kernel
        .apply(request(
            generation,
            [saved("main.recite", ":: main\n-> absent\n")],
            [],
        ))
        .expect("missing target replacement accepted");
    let QueryResult::Ready(recited) = kernel.snapshot().navigate(&key("main.recite"), position)
    else {
        panic!("missing navigation is ready");
    };
    assert!(matches!(
        recited,
        recite_compiler::NavigationResult::Missing
    ));
}

#[test]
fn missing_ids_keep_typed_identity_and_exact_insertion_points_across_crlf_utf8() {
    let source = ":: start\r\n> \r\n  💬\r\n? \r\n  Pick this\r\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [saved("unicode.recite", source)],
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
        assert!(matches!(stable.source_id(), SourceId::Missing));
        assert!(stable.source_id_span().is_none());
    }
    assert_eq!(
        document.summary().stable_ids()[0]
            .insertion_span()
            .expect("parser insertion span")
            .start,
        SourcePosition::new(2, 3).expect("valid insertion position")
    );
    assert_eq!(
        document.summary().stable_ids()[1]
            .insertion_span()
            .expect("parser insertion span")
            .start,
        SourcePosition::new(4, 3).expect("valid insertion position")
    );
}

#[test]
fn typed_recovery_marks_only_unsafe_semantic_classes() {
    let cases = [
        (
            "if.recite",
            ":: start\n:if broken(\n  -> missing\n",
            "condition",
        ),
        (
            "match.recite",
            ":: start\n:match mood()\n  :case\n    -> missing\n",
            "condition",
        ),
        (
            "else.recite",
            ":: start\n:else\n  -> missing\n",
            "condition",
        ),
        (
            "case.recite",
            ":: start\n:case orphan\n  -> missing\n",
            "condition",
        ),
        ("body.recite", ":: start\n  -> kept\n   -> missing\n", "ast"),
        (
            "id.recite",
            "> line@11111111111111111111\n:: start\n",
            "stable",
        ),
    ];
    for (name, source, class) in cases {
        let mut kernel = AuthoringKernel::new();
        kernel
            .apply(request(
                SnapshotGeneration::initial(),
                [saved(name, source), saved("target.recite", ":: missing\n")],
                [],
            ))
            .expect("recovery fixture accepted");
        let document = kernel
            .snapshot()
            .document(&key(name))
            .expect("fixture present");
        let participation = document.participation();
        assert!(!match class {
            "condition" => participation.condition_functions().is_complete(),
            "stable" => participation.stable_ids().is_complete(),
            _ => participation.ast_structure().is_complete(),
        });
        assert!(
            kernel
                .snapshot()
                .document(&key("target.recite"))
                .expect("target present")
                .diagnostics()
                .iter()
                .all(|diagnostic| diagnostic.code.as_str() != "RECITE_VALIDATE007")
        );
    }
}
