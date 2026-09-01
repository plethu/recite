#![cfg(test)]

use recite_compiler::{
    AuthoringEditError, AuthoringKernel, AuthoringRequest, DocumentVersion, OpenDocument,
    QueryClass, SavedDocument, SnapshotGeneration, SourceFingerprint,
};
use recite_core::{DocumentKey, SourcePosition};

fn key(value: &str) -> DocumentKey {
    match DocumentKey::new(value.to_owned()) {
        Ok(key) => key,
        Err(error) => panic!("invalid test document key {value:?}: {error:?}"),
    }
}

fn position(line: u32, column: u32) -> SourcePosition {
    match SourcePosition::new(line, column) {
        Ok(position) => position,
        Err(error) => panic!("invalid test position {line}:{column}: {error:?}"),
    }
}

fn kernel(documents: impl IntoIterator<Item = SavedDocument>) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    match kernel.apply(AuthoringRequest::new(
        SnapshotGeneration::initial(),
        documents,
        [],
    )) {
        Ok(_) => kernel,
        Err(error) => panic!("test source was rejected: {error:?}"),
    }
}

#[test]
fn stub_empty_target_has_no_leading_blank() {
    let source = ":: source\n-> target.recite::missing\n";
    let target = key("target.recite");
    let plan = kernel([
        SavedDocument::new(key("main.recite"), source),
        SavedDocument::new(target.clone(), ""),
    ])
    .snapshot()
    .plan_create_block_stub(&key("main.recite"), position(2, 23));
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => panic!("empty target should be planable: {error:?}"),
    };
    assert_eq!(plan.edits()[0].document(), &target);
    assert_eq!(plan.edits()[0].replacement(), ":: missing\n");
}

#[test]
fn stub_preserves_eof_style_for_no_final_newline_and_same_document() {
    let target = key("target.recite");
    let no_final_newline = kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> target.recite::missing\n"),
        SavedDocument::new(target.clone(), ":: target"),
    ]);
    let plan = match no_final_newline
        .snapshot()
        .plan_create_block_stub(&key("main.recite"), position(2, 23))
    {
        Ok(plan) => plan,
        Err(error) => panic!("target without final newline should be planable: {error:?}"),
    };
    assert_eq!(plan.edits()[0].replacement(), "\n:: missing\n");

    let local = kernel([SavedDocument::new(
        key("main.recite"),
        ":: source\n-> missing\n",
    )]);
    let plan = match local
        .snapshot()
        .plan_create_block_stub(&key("main.recite"), position(2, 9))
    {
        Ok(plan) => plan,
        Err(error) => panic!("same-document stub should be planable: {error:?}"),
    };
    assert_eq!(plan.edits()[0].document(), &key("main.recite"));
    assert_eq!(plan.edits()[0].replacement(), ":: missing\n");
}

#[test]
fn invalid_qualified_stub_target_never_falls_back_to_source_document() {
    let source = ":: source\n-> ../target.recite::missing\n";
    let kernel = kernel([SavedDocument::new(key("main.recite"), source)]);
    let snapshot = kernel.snapshot();
    let site = match snapshot.completion_site(&key("main.recite"), position(2, 23)) {
        Some(site) => site,
        None => panic!("invalid qualified reference should still have a typed site"),
    };
    assert!(matches!(
        site.block_target_resolution(),
        Some(recite_compiler::BlockTarget::InvalidQualified { target })
            if target == "../target.recite"
    ));
    assert!(matches!(
        snapshot.plan_create_block_stub(&key("main.recite"), position(2, 23)),
        Err(AuthoringEditError::InvalidTargetDocument { document })
            if document == "../target.recite"
    ));
}

#[test]
fn invalid_alias_stub_target_is_not_treated_as_a_local_reference() {
    let source = ":: source\n-> ./target.recite::missing\n";
    let kernel = kernel([SavedDocument::new(key("main.recite"), source)]);
    let snapshot = kernel.snapshot();
    assert!(matches!(
        snapshot.plan_create_block_stub(&key("main.recite"), position(2, 23)),
        Err(AuthoringEditError::InvalidTargetDocument { document })
            if document == "./target.recite"
    ));
}

#[test]
fn non_bmp_definition_range_is_end_exclusive() {
    let kernel = kernel([SavedDocument::new(key("main.recite"), ":: 😀\n")]);
    let snapshot = kernel.snapshot();
    let plan = match snapshot.plan_rename_block(&key("main.recite"), position(1, 4), "target") {
        Ok(plan) => plan,
        Err(error) => panic!("non-BMP block should be renameable: {error:?}"),
    };
    assert_eq!(plan.edits().len(), 1);
    assert_eq!(plan.edits()[0].range().start(), position(1, 4));
    assert_eq!(plan.edits()[0].range().end(), position(1, 5));
}

#[test]
fn duplicate_block_ownership_refuses_stable_id_planning() {
    let kernel = kernel([SavedDocument::new(
        key("main.recite"),
        ":: source\n> line\n:: source\n> other\n",
    )]);
    let snapshot = kernel.snapshot();
    assert!(matches!(
        snapshot.plan_insert_missing_ids(),
        Err(AuthoringEditError::AmbiguousBlock { block }) if block.as_str() == "source"
    ));
}

#[test]
fn stable_id_anchor_inputs_remain_known_and_combined_namespace_collides() {
    let snapshot = kernel([
        SavedDocument::new(key("main.recite"), ":: source\n> line\n"),
        SavedDocument::new(
            key("other.recite"),
            ":: other\n> line\n  ? choice@955f817fa5db1ac97e5b\n",
        ),
    ]);
    let plan = match snapshot.snapshot().plan_insert_missing_ids() {
        Ok(plan) => plan,
        Err(error) => panic!("stable ID should be planable: {error:?}"),
    };
    let Some(edit) = plan
        .edits()
        .iter()
        .find(|edit| edit.document() == &key("main.recite"))
    else {
        panic!("main stable ID edit is missing");
    };
    assert_eq!(edit.replacement(), "@dfdb708cf91598dfe6e5");
}

#[test]
fn stale_version_and_source_fingerprint_are_independent_preconditions() {
    let source = ":: source\n-> missing\n";
    let plan = kernel([SavedDocument::new(key("main.recite"), source)])
        .snapshot()
        .plan_create_block_stub(&key("main.recite"), position(2, 9));
    let plan = match plan {
        Ok(plan) => plan,
        Err(error) => panic!("local stub should be planable: {error:?}"),
    };
    let mut changed_version = AuthoringKernel::new();
    let generation = changed_version.snapshot().generation();
    match changed_version.apply(AuthoringRequest::new(
        generation,
        [],
        [OpenDocument::new(
            key("main.recite"),
            DocumentVersion::new(9),
            source,
        )],
    )) {
        Ok(_) => {}
        Err(error) => panic!("version overlay should be accepted: {error:?}"),
    }
    assert!(matches!(
        plan.validate(changed_version.snapshot()),
        Err(AuthoringEditError::StaleDocumentVersion { .. })
    ));

    let changed_source = kernel([SavedDocument::new(
        key("main.recite"),
        ":: source\n-> other\n",
    )]);
    assert!(matches!(
        plan.validate(changed_source.snapshot()),
        Err(AuthoringEditError::StaleSource { .. })
    ));
    assert!(SourceFingerprint::for_source(source).matches_source(source));
}

#[test]
fn incomplete_cross_document_reference_coverage_refuses_rename() {
    let kernel = kernel([
        SavedDocument::new(key("main.recite"), ":: target\n"),
        SavedDocument::new(key("hidden.recite"), ":: hidden\n-> main.recite::\n"),
    ]);
    let snapshot = kernel.snapshot();
    assert!(matches!(
        snapshot.plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::Incomplete {
            class: QueryClass::BlockReferences,
            ..
        })
    ));
}

#[test]
fn plan_order_is_independent_of_request_document_order() {
    let first = kernel([
        SavedDocument::new(key("z.recite"), ":: z\n> line\n"),
        SavedDocument::new(key("a.recite"), ":: a\n> line\n"),
    ]);
    let second = kernel([
        SavedDocument::new(key("a.recite"), ":: a\n> line\n"),
        SavedDocument::new(key("z.recite"), ":: z\n> line\n"),
    ]);
    let first_plan = match first.snapshot().plan_insert_missing_ids() {
        Ok(plan) => plan,
        Err(error) => panic!("first order should be planable: {error:?}"),
    };
    let second_plan = match second.snapshot().plan_insert_missing_ids() {
        Ok(plan) => plan,
        Err(error) => panic!("second order should be planable: {error:?}"),
    };
    assert_eq!(first_plan, second_plan);
}

#[test]
fn rename_includes_duplicate_references_without_touching_prose() {
    let source = ":: target\n-> target\n# target prose\n-> target\n";
    let snapshot = kernel([SavedDocument::new(key("main.recite"), source)]);
    let plan =
        match snapshot
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed")
        {
            Ok(plan) => plan,
            Err(error) => panic!("duplicate references should be planable: {error:?}"),
        };
    assert_eq!(plan.edits().len(), 3);
    let replacements = plan
        .edits()
        .iter()
        .map(|edit| edit.replacement())
        .collect::<Vec<_>>();
    assert_eq!(replacements, ["renamed", "renamed", "renamed"]);
}
