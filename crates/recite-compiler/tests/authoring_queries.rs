#![cfg(test)]

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, MetadataScalar, MetadataValue, QueryResult, SavedDocument,
    SemanticFact, SnapshotGeneration, SymbolIdentity, SymbolKind, SymbolQueryOptions, SymbolRole,
};
use recite_core::{DocumentKey, SourceId, SourcePosition};

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("test key is valid")
}
fn saved(name: &str, text: &str) -> SavedDocument {
    SavedDocument::new(key(name), text)
}
fn request(
    generation: SnapshotGeneration,
    saved: impl IntoIterator<Item = SavedDocument>,
) -> AuthoringRequest {
    AuthoringRequest::new(generation, saved, [])
}

#[test]
fn summaries_and_queries_preserve_typed_values_spans_and_navigation() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(request(
            SnapshotGeneration::initial(),
            [
                saved(
                    "dialogue/main.recite",
                    concat!(
                        ":: start scene=opening retries=3 values=[one, \"two\"]\n",
                        "> intro@11111111111111111111\n",
                        "  Hello.\n",
                        "  -> dialogue/target.recite::finish\n"
                    ),
                ),
                saved("dialogue/target.recite", ":: finish\n"),
            ],
        ))
        .expect("query fixture accepted");
    let main = kernel
        .snapshot()
        .document(&key("dialogue/main.recite"))
        .expect("main document exists");
    assert_eq!(
        main.summary().blocks()[0]
            .id_span()
            .expect("parser span")
            .start
            .column(),
        4
    );
    assert_eq!(
        main.summary().block_references()[0]
            .file_span()
            .map(|s| s.start.column()),
        Some(6)
    );
    assert_eq!(
        main.summary().block_references()[0]
            .block_id_span()
            .map(|s| s.start.column()),
        Some(30)
    );
    assert!(main.summary().metadata().iter().any(|metadata| matches!(
        metadata.value(),
        MetadataValue::Scalar(MetadataScalar::Integer(3))
    )));
    assert!(
        main.summary()
            .metadata()
            .iter()
            .all(|metadata| metadata.source_span().is_some())
    );
    assert!(main.summary().stable_ids().iter().any(|stable| matches!(
        stable.source_id(),
        SourceId::Frozen { .. }
    )
        && stable.source_id_span().is_some()));
    let position = SourcePosition::new(4, 30).expect("valid source position");
    let QueryResult::Ready(navigation) = kernel
        .snapshot()
        .navigate(&key("dialogue/main.recite"), position)
    else {
        panic!("qualified target navigation is ready");
    };
    let recite_compiler::NavigationResult::Unique(declaration) = navigation else {
        panic!("qualified target has one declaration");
    };
    assert_eq!(declaration.document().as_str(), "dialogue/target.recite");
    assert_eq!(declaration.span().start.column(), 4);
    let QueryResult::Ready(completions) = kernel
        .snapshot()
        .completions(&key("dialogue/main.recite"), position)
    else {
        panic!("target completion is ready");
    };
    assert_eq!(completions.len(), 1);
    assert_eq!(completions[0].replace_span().start.column(), 30);
    let QueryResult::Ready(symbols) = kernel
        .snapshot()
        .symbols(&key("dialogue/main.recite"), SymbolQueryOptions::default())
    else {
        panic!("symbol query is ready");
    };
    assert!(
        symbols
            .iter()
            .any(|symbol| symbol.kind() == SymbolKind::BlockReference
                && symbol.role() == SymbolRole::Reference
                && matches!(symbol.identity(), SymbolIdentity::Block(_)))
    );
    let QueryResult::Ready(without_declarations) = kernel
        .snapshot()
        .symbols(&key("dialogue/main.recite"), SymbolQueryOptions::new(false))
    else {
        panic!("filtered symbol query is ready");
    };
    assert!(
        without_declarations
            .iter()
            .all(|symbol| symbol.role() != SymbolRole::Definition)
    );
    let metadata_value = main
        .summary()
        .metadata()
        .iter()
        .find(|metadata| metadata.key() == "scene")
        .and_then(|metadata| metadata.value_span())
        .expect("metadata value span");
    let QueryResult::Ready(hover) = kernel
        .snapshot()
        .hover(&key("dialogue/main.recite"), metadata_value.start)
    else {
        panic!("metadata value hover is ready");
    };
    assert!(
        matches!(hover.facts(), [SemanticFact::MetadataValue(MetadataValue::Scalar(MetadataScalar::Symbol(value)))] if value == "opening")
    );
    let QueryResult::Ready(unsupported) = kernel.snapshot().navigate(
        &key("dialogue/main.recite"),
        SourcePosition::new(1, 10).expect("metadata key position"),
    ) else {
        panic!("metadata navigation is represented");
    };
    assert!(matches!(
        unsupported,
        recite_compiler::NavigationResult::Unsupported
    ));
}
