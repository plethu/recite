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
