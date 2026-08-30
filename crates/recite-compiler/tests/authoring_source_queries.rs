#![cfg(test)]

use std::collections::BTreeSet;

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, CompletionCandidateDetail, CompletionCandidateKind,
    MetadataValue, QueryResult, SavedDocument, SemanticFact, SnapshotGeneration,
};
use recite_core::{
    AvailabilityReasonDefinition, ConditionDefinition, DocumentKey, FlatMetadataDomain,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, ProjectSchema,
    ProjectionQueryFunctionDefinition, SchemaPresentationProjectorDefinition,
    SchemaProjectionSelector, SchemaTypeRef, SourcePosition, SpeakerDefinition,
};

#[path = "authoring_source_queries/typed.rs"]
mod typed_queries;

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("valid document key")
}

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("valid source position")
}

fn fixture() -> AuthoringKernel {
    let mut schema = ProjectSchema::empty_v1();
    schema.speakers.insert(
        "hazel".to_owned(),
        SpeakerDefinition {
            display_name: Some("Hazel".to_owned()),
        },
    );
    schema.conditions.insert(
        "knows_secret".to_owned(),
        ConditionDefinition {
            params: Vec::new(),
            returns: recite_core::ConditionReturnType::Bool,
            availability_reason: None,
        },
    );
    schema.availability_reasons.insert(
        recite_core::AvailabilityReasonId::new("innkeeper_trust_hint").expect("reason ID"),
        AvailabilityReasonDefinition {
            template: "Trust is too low.".to_owned(),
            params: Vec::new(),
            origin: None,
        },
    );
    schema.metadata.insert(
        "mood".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("moods".to_owned()),
        },
    );
    schema.metadata_domains.insert(
        "moods".to_owned(),
        MetadataDomainDefinition::Flat(FlatMetadataDomain::default()),
    );
    schema.projection_queries.insert(
        "is_ready".to_owned(),
        ProjectionQueryFunctionDefinition {
            params: Vec::new(),
            returns: SchemaTypeRef::Bool,
            max_calls_per_event: None,
        },
    );
    schema.presentation_projectors.insert(
        "hud".to_owned(),
        SchemaPresentationProjectorDefinition {
            candidates: SchemaProjectionSelector::RuntimeEvent {
                kind: "dialogue".to_owned(),
            },
            inputs: Vec::new(),
            queries: Default::default(),
            outputs: Default::default(),
        },
    );
    AuthoringKernel::with_schema(schema)
}

fn source() -> SavedDocument {
    SavedDocument::new(
        key("main.recite"),
        concat!(
            ":: start\n",
            "> line@11111111111111111111 speaker=hazel mood=\n",
            "  ordinary prose\n",
            "  :if knows_\n",
        ),
    )
}

#[test]
fn completion_derives_site_from_source_and_cursor() {
    let mut kernel = fixture();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [source()],
            [],
        ))
        .expect("source accepted");
    let key = key("main.recite");

    let QueryResult::Ready(speakers) = kernel.snapshot().complete(&key, position(2, 38)) else {
        panic!("speaker site is recognized");
    };
    assert!(speakers.iter().any(|candidate| {
        candidate.name() == "hazel"
            && candidate.kind() == CompletionCandidateKind::Speaker
            && matches!(
                candidate.detail(),
                CompletionCandidateDetail::Speaker { display_name: Some(name) } if name == "Hazel"
            )
    }));

    let QueryResult::Ready(empty) = kernel.snapshot().complete(&key, position(2, 48)) else {
        panic!("empty metadata domain is a ready empty result");
    };
    assert!(empty.is_empty(), "unexpected candidates: {empty:?}");

    assert!(matches!(
        kernel.snapshot().complete(&key, position(3, 8)),
        QueryResult::NoMatch
    ));
    let QueryResult::Ready(conditions) = kernel.snapshot().complete(&key, position(4, 12)) else {
        panic!("condition site is recognized");
    };
    assert_eq!(conditions[0].name(), "knows_secret");
    assert_eq!(conditions[0].replace_span().start.column(), 7);
    assert_eq!(
        conditions[0]
            .replace_span()
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(11)
    );
}

#[test]
fn hover_resolves_the_token_without_a_caller_context() {
    let mut kernel = fixture();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [source()],
            [],
        ))
        .expect("source accepted");
    let key = key("main.recite");
    let hover_result = kernel.snapshot().hover(&key, position(2, 39));
    let QueryResult::Ready(hover) = hover_result.clone() else {
        panic!("speaker hover is ready: {hover_result:?}");
    };
    assert!(matches!(
        hover.facts(),
        [SemanticFact::SchemaCandidate { name, .. }] if name == "hazel"
    ));
    assert!(matches!(
        kernel.snapshot().hover(&key, position(3, 8)),
        QueryResult::NoMatch
    ));
}

#[test]
fn references_are_key_scoped_and_include_declarations_is_typed() {
    let mut kernel = fixture();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: start\n-> target\n:: target\n",
            )],
            [],
        ))
        .expect("source accepted");
    let result = kernel.snapshot().references(
        &key("main.recite"),
        position(2, 4),
        recite_compiler::SymbolQueryOptions::new(false),
    );
    let QueryResult::Ready(references) = result else {
        panic!("references are ready");
    };
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].document().as_str(), "main.recite");
}

#[test]
fn array_metadata_retains_element_spans() {
    let mut kernel = AuthoringKernel::new();
    let source = concat!(r#":: start tags=["a\"b", "µ"]"#, "\n");
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(key("main.recite"), source)],
            [],
        ))
        .expect("source accepted");
    let metadata = &kernel.snapshot().documents()[0].summary().metadata()[0];
    assert_eq!(metadata.value_element_spans().len(), 2);
    assert!(matches!(metadata.value(), MetadataValue::Array(values) if values.len() == 2));
    assert_eq!(metadata.value_element_spans()[0].start.column(), 16);
    assert_eq!(
        metadata.value_element_spans()[0]
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(21)
    );
    assert_eq!(metadata.value_element_spans()[1].start.column(), 24);
    assert_eq!(
        metadata.value_element_spans()[1]
            .end
            .as_ref()
            .map(|end| end.column()),
        Some(26)
    );
}

#[test]
fn block_completion_and_hover_keep_project_scope_and_source_identity() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("main.recite"), ":: local\n-> lo\n"),
                SavedDocument::new(key("other.recite"), ":: local\n:: other\n"),
            ],
            [],
        ))
        .expect("project accepted");
    let main = key("main.recite");
    let QueryResult::Ready(candidates) = kernel.snapshot().complete(&main, position(2, 7)) else {
        panic!("unqualified completion is ready");
    };
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| candidate.name())
            .collect::<Vec<_>>(),
        ["local"]
    );
    let QueryResult::Ready(hover) = kernel.snapshot().hover(&main, position(2, 4)) else {
        panic!("block reference hover is ready");
    };
    assert_eq!(
        hover.location().kind(),
        recite_compiler::SymbolKind::BlockReference
    );
    assert!(matches!(hover.facts(), [SemanticFact::Reference]));
}

#[test]
fn references_without_declarations_do_not_require_incomplete_definitions() {
    let mut kernel = AuthoringKernel::new();
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [
                SavedDocument::new(key("main.recite"), ":: start\n-> target\n"),
                SavedDocument::new(key("target.recite"), "::\n"),
            ],
            [],
        ))
        .expect("recoverable project accepted");
    let result = kernel.snapshot().references(
        &key("main.recite"),
        position(2, 4),
        recite_compiler::SymbolQueryOptions::new(false),
    );
    let QueryResult::Ready(references) = result else {
        panic!("reference-only query does not consume definitions: {result:?}");
    };
    assert_eq!(references.len(), 1);
}
