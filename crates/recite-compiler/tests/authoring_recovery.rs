#![cfg(test)]

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, QueryClass, SavedDocument, SnapshotGeneration,
    SymbolQueryOptions,
};
use recite_core::DocumentKey;

fn key() -> DocumentKey {
    DocumentKey::new("recovery.recite").expect("valid key")
}

fn recovery_for(source: &str) -> recite_compiler::ValidationParticipation {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key(), source)],
            [],
        ))
        .expect("recovery fixture accepted");
    kernel.snapshot().documents()[0].participation()
}

#[test]
fn dropped_or_ambiguous_regions_never_claim_ast_completeness() {
    let fixtures = [
        ":: start\n:if broken(\n  -> missing\n",
        ":: start\n:match mood()\n  :case\n    -> missing\n",
        ":: start\n:else\n  -> missing\n",
        ":: start\n:case orphan\n  -> missing\n",
        ":: start\n  -> kept\n   -> missing\n",
        ":: start\n  -> next\n  prose after statement\n",
        "? choice@11111111111111111111 if\n",
        "::\n",
        ":: start\n->\n",
        ":: start\n! broken\n",
    ];
    for source in fixtures {
        let participation = recovery_for(source);
        assert!(
            !participation.ast_structure().is_complete(),
            "fixture was marked complete: {source:?}"
        );
    }
}

#[test]
fn targeted_recovery_marks_only_the_affected_class() {
    let metadata = recovery_for(":: start\n> line@11111111111111111111 bind=(name:string=$)\n");
    assert!(!metadata.metadata().is_complete());
    assert!(metadata.block_references().is_complete());
}

#[test]
fn syntax_recovery_marks_stable_ids_incomplete() {
    let recovery = recovery_for(":: start\n> line@bad\n  Recoverable ID.\nsyntax error\n");
    assert!(!recovery.stable_ids().is_complete());
    assert!(!recovery.block_references().is_complete());
}

#[test]
fn invalid_ids_do_not_suppress_independent_stable_id_diagnostics() {
    let source = concat!(
        ":: start default\n",
        "> malformed@bad\n",
        "  Malformed ID.\n",
        "> duplicate@11111111111111111111\n",
        "  First duplicate.\n",
        "> duplicate@11111111111111111111\n",
        "  Second duplicate.\n",
        ">\n",
        "  Missing ID.\n",
    );
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key(), source)],
            [],
        ))
        .expect("recoverable source request accepted");

    let document = &kernel.snapshot().documents()[0];
    assert!(document.participation().stable_ids().is_complete());
    let codes: Vec<_> = document
        .diagnostics()
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect();
    assert!(codes.contains(&"RECITE_ID001"), "missing ID: {codes:?}");
    assert!(codes.contains(&"RECITE_ID003"), "duplicate ID: {codes:?}");
    assert!(codes.contains(&"RECITE_ID007"), "malformed ID: {codes:?}");
}

#[test]
fn unrelated_clean_files_keep_their_local_participation() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key(), ":: start\n:if broken(\n"),
                SavedDocument::new(
                    DocumentKey::new("clean.recite").expect("valid key"),
                    ":: clean\n-> clean\n",
                ),
            ],
            [],
        ))
        .expect("project accepted");
    let clean = kernel
        .snapshot()
        .document(&DocumentKey::new("clean.recite").expect("valid key"))
        .expect("clean document");
    assert!(clean.participation().block_definitions().is_complete());
    assert!(clean.participation().block_references().is_complete());
}

#[test]
fn malformed_diagnostics_are_local_and_recovery_preserves_clean_validation() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(
                    DocumentKey::new("clean.recite").expect("valid key"),
                    ":: start\n>\n  A line without an ID.\n-> malformed.recite::missing\n",
                ),
                SavedDocument::new(
                    DocumentKey::new("malformed.recite").expect("valid key"),
                    "oops\n",
                ),
            ],
            [],
        ))
        .expect("recoverable source request accepted");
    let clean = kernel
        .snapshot()
        .document(&DocumentKey::new("clean.recite").expect("valid key"))
        .expect("clean document is present");
    let malformed = kernel
        .snapshot()
        .document(&DocumentKey::new("malformed.recite").expect("valid key"))
        .expect("malformed document is present");
    assert!(
        malformed
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str().starts_with("RECITE_PARSE"))
    );
    assert!(!malformed.participation().ast_structure().is_complete());
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_ID001")
    );

    kernel
        .apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [
                SavedDocument::new(
                    DocumentKey::new("clean.recite").expect("valid key"),
                    ":: start\n> line@11111111111111111111\n  A line with an ID.\n-> malformed.recite::missing\n",
                ),
                SavedDocument::new(
                    DocumentKey::new("malformed.recite").expect("valid key"),
                    ":: known\n",
                ),
            ],
            [],
        ))
        .expect("complete replacement accepted");
    let clean = kernel
        .snapshot()
        .document(&DocumentKey::new("clean.recite").expect("valid key"))
        .expect("clean document remains present");
    assert!(
        clean
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "RECITE_VALIDATE007")
    );
}

#[test]
fn symbol_readiness_reports_the_actual_incomplete_classes() {
    let cases = [
        ("::\n", QueryClass::BlockDefinitions),
        (":: start\n->\n", QueryClass::BlockReferences),
        (
            ":: start\n> line@bad\n  Recoverable ID.\nsyntax error\n",
            QueryClass::StableIds,
        ),
        (
            ":: start\n> line@11111111111111111111 bind=(name:string=$)\n",
            QueryClass::Metadata,
        ),
        (":: start\n:if broken(\n", QueryClass::ConditionFunctions),
        (":: start\n! broken\n", QueryClass::EffectFunctions),
    ];
    for (source, class) in cases {
        let mut kernel = AuthoringKernel::new();
        kernel
            .apply(AuthoringRequest::new(
                SnapshotGeneration::initial(),
                [SavedDocument::new(key(), source)],
                [],
            ))
            .expect("readiness fixture accepted");
        let result = kernel
            .snapshot()
            .symbols(&key(), SymbolQueryOptions::default());
        assert!(
            result
                .unavailable_reasons()
                .contains(&recite_compiler::QueryUnavailableReason::Incomplete(class))
        );
        let project_result = kernel
            .snapshot()
            .project_symbols(SymbolQueryOptions::default());
        assert!(
            project_result
                .unavailable_reasons()
                .contains(&recite_compiler::QueryUnavailableReason::Incomplete(class))
        );
    }

    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key(), "::\n")],
            [],
        ))
        .expect("definition recovery fixture accepted");
    let options = SymbolQueryOptions::new(false);
    let local = kernel.snapshot().symbols(&key(), options);
    let project = kernel.snapshot().project_symbols(options);
    let reason = recite_compiler::QueryUnavailableReason::Incomplete(QueryClass::BlockDefinitions);
    assert!(!local.unavailable_reasons().contains(&reason));
    assert!(!project.unavailable_reasons().contains(&reason));
}
