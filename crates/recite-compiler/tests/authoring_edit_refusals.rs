#![cfg(test)]

use recite_compiler::{
    AuthoringEditError, AuthoringKernel, AuthoringRequest, QueryClass, SavedDocument,
    SnapshotGeneration,
};
use recite_core::{DocumentKey, SourcePosition};

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test document key is valid")
}

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("test source position is valid")
}

fn kernel(source: &str) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("source accepted");
    kernel
}

fn project_kernel(documents: impl IntoIterator<Item = SavedDocument>) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            documents,
            [],
        ))
        .expect("source accepted");
    kernel
}

fn incomplete_project_kernel(
    documents: impl IntoIterator<Item = SavedDocument>,
) -> AuthoringKernel {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(
            AuthoringRequest::new(SnapshotGeneration::initial(), documents, [])
                .with_project_completeness(false),
        )
        .expect("incomplete project source accepted");
    kernel
}

#[test]
fn rename_refuses_ambiguous_definitions_from_navigation() {
    let kernel = kernel(":: target\n:: target\n");
    assert!(matches!(
        kernel
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::AmbiguousSymbol { .. })
    ));
}

#[test]
fn rename_refuses_partial_reference_query() {
    let kernel = kernel(":: target\n-> target\n->\n");
    assert!(matches!(
        kernel
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::Incomplete {
            class: QueryClass::BlockReferences,
            ..
        })
    ));
}

#[test]
fn stub_refuses_partial_symbol_query_before_materialising_an_edit() {
    let kernel = kernel(":: source\n-> missing\n->\n");
    assert!(matches!(
        kernel
            .snapshot()
            .plan_create_block_stub(&key("main.recite"), position(2, 4)),
        Err(AuthoringEditError::Incomplete { .. })
    ));
}

#[test]
fn rename_refuses_destination_defined_in_another_document() {
    let kernel = project_kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> source\n"),
        SavedDocument::new(key("other.recite"), ":: renamed\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::DestinationCollision { document, block })
            if document == key("main.recite") && block.as_str() == "renamed"
    ));
}

#[test]
fn qualified_stub_refuses_name_defined_in_another_document() {
    let kernel = project_kernel([
        SavedDocument::new(
            key("main.recite"),
            ":: source\n-> target.recite::missing\n:: missing\n",
        ),
        SavedDocument::new(key("target.recite"), ":: target\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_create_block_stub(&key("main.recite"), position(2, 23)),
        Err(AuthoringEditError::TargetAlreadyExists { document, block })
            if document == key("target.recite") && block.as_str() == "missing"
    ));
}

#[test]
fn incomplete_project_refuses_rename_without_known_collision() {
    let kernel = incomplete_project_kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> source\n"),
        SavedDocument::new(key("other.recite"), ":: other\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::Incomplete {
            document,
            class: QueryClass::BlockDefinitions,
        }) if document == key("main.recite")
    ));
}

#[test]
fn incomplete_project_refuses_qualified_stub_without_known_collision() {
    let kernel = incomplete_project_kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> target.recite::missing\n"),
        SavedDocument::new(key("target.recite"), ":: target\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_create_block_stub(&key("main.recite"), position(2, 23)),
        Err(AuthoringEditError::Incomplete {
            class: QueryClass::BlockDefinitions,
            ..
        })
    ));
}

#[test]
fn unrelated_incomplete_block_definitions_refuse_rename() {
    let kernel = project_kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> source\n"),
        SavedDocument::new(key("other.recite"), ":: other\n"),
        SavedDocument::new(key("broken.recite"), "::\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_rename_block(&key("main.recite"), position(1, 4), "renamed"),
        Err(AuthoringEditError::Incomplete {
            class: QueryClass::BlockDefinitions,
            ..
        })
    ));
}

#[test]
fn unrelated_incomplete_block_definitions_refuse_qualified_stub() {
    let kernel = project_kernel([
        SavedDocument::new(key("main.recite"), ":: source\n-> target.recite::missing\n"),
        SavedDocument::new(key("target.recite"), ":: target\n"),
        SavedDocument::new(key("broken.recite"), "::\n"),
    ]);
    assert!(matches!(
        kernel
            .snapshot()
            .plan_create_block_stub(&key("main.recite"), position(2, 23)),
        Err(AuthoringEditError::Incomplete {
            class: QueryClass::BlockDefinitions,
            ..
        })
    ));
}

#[test]
fn single_stable_id_selection_does_not_capture_later_header_columns() {
    let kernel = kernel(":: source\n> speaker=é\n  Text.\n");
    assert!(
        kernel
            .snapshot()
            .plan_insert_missing_id(&key("main.recite"), position(2, 1))
            .is_ok()
    );
    assert!(matches!(
        kernel
            .snapshot()
            .plan_insert_missing_id(&key("main.recite"), position(2, 10)),
        Err(AuthoringEditError::NoSymbol { .. })
    ));
}
