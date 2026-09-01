use recite_compiler::{AuthoringKernel, AuthoringRequest, SavedDocument, SnapshotGeneration};
use recite_core::DocumentKey;

use crate::edit_projection::{EditDocument, project_plan};
use crate::tests::support::uri;

pub(super) fn projector_refuses_mismatched_precondition_documents() {
    let source_key = key("main.recite");
    let target_key = key("target.recite");
    let source = ":: source\n-> target.recite::missing\n";
    let target = ":: target\n";
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(source_key.clone(), source),
                SavedDocument::new(target_key.clone(), target),
            ],
            [],
        ))
        .expect("source accepted");
    let snapshot = kernel.snapshot();
    let plan = snapshot
        .plan_create_block_stub(&source_key, position(2, 23))
        .expect("stub is planable");
    let source_uri = uri("file:///workspace/main.recite");
    let target_uri = uri("file:///workspace/target.recite");
    let mismatched_target = ":: changed\n";
    let documents = [
        EditDocument {
            key: &source_key,
            uri: &source_uri,
            text: source,
            layer: recite_compiler::DocumentLayer::Saved,
            version: None,
        },
        EditDocument {
            key: &target_key,
            uri: &target_uri,
            text: mismatched_target,
            layer: recite_compiler::DocumentLayer::Saved,
            version: None,
        },
    ];
    assert!(project_plan(&plan, snapshot, &documents[..1]).is_none());
    assert!(project_plan(&plan, snapshot, &documents).is_none());

    let duplicate_uri_documents = [
        documents[0],
        EditDocument {
            key: &target_key,
            uri: &source_uri,
            text: target,
            layer: recite_compiler::DocumentLayer::Saved,
            version: None,
        },
    ];
    assert!(project_plan(&plan, snapshot, &duplicate_uri_documents).is_none());
}

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value.to_owned()).expect("test document key is valid")
}

fn position(line: u32, column: u32) -> recite_core::SourcePosition {
    recite_core::SourcePosition::new(line, column).expect("test source position is valid")
}
