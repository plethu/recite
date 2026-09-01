#![cfg(test)]

use std::collections::BTreeSet;

use recite_compiler::{
    AuthoringKernel, AuthoringRequest, CompletionCandidateKind, QueryResult, SavedDocument,
    SnapshotGeneration,
};
use recite_core::{
    ContextualMetadataDomain, DocumentKey, MetadataContextSelector, MetadataDefinition,
    MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy, ProjectSchema,
    ProjectionQueryFunctionDefinition, SchemaPresentationProjectorDefinition,
    SchemaProjectionSelector, SchemaTypeRef, SourcePosition,
};

fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).expect("valid document key")
}

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).expect("valid source position")
}

#[test]
fn projection_enumeration_is_explicit_and_typed() {
    let mut schema = ProjectSchema::empty_v1();
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
    let kernel = AuthoringKernel::with_schema(schema);
    let QueryResult::Ready(candidates) = kernel.snapshot().projection_candidates("hud") else {
        panic!("known projector is available");
    };
    assert!(candidates.iter().any(|candidate| {
        candidate.name() == "hud"
            && candidate.kind() == CompletionCandidateKind::ProjectionProjector
    }));
    assert!(matches!(
        kernel.snapshot().projection_candidates("missing"),
        QueryResult::NoMatch
    ));
}

#[test]
fn contextual_metadata_uses_canonical_inherited_and_key_context() {
    let mut schema = ProjectSchema::empty_v1();
    for (name, domain) in [("tone", "tone_by_subject"), ("voice", "voice_by_speaker")] {
        schema.metadata.insert(
            name.to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Symbol,
                repeatable: false,
                domain: Some(domain.to_owned()),
            },
        );
    }
    schema.metadata_domains.insert(
        "tone_by_subject".to_owned(),
        MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
            selector: MetadataContextSelector::MetadataKey("subject".to_owned()),
            values_by_context: BTreeSet::from([(
                "warm".to_owned(),
                BTreeSet::from(["market".to_owned()]),
            )])
            .into_iter()
            .collect(),
            missing_context: MissingMetadataContextPolicy::Diagnostic,
            provenance: Default::default(),
        }),
    );
    schema.metadata_domains.insert(
        "voice_by_speaker".to_owned(),
        MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
            selector: MetadataContextSelector::FieldSpeaker,
            values_by_context: BTreeSet::from([(
                "hazel".to_owned(),
                BTreeSet::from(["soft".to_owned()]),
            )])
            .into_iter()
            .collect(),
            missing_context: MissingMetadataContextPolicy::Diagnostic,
            provenance: Default::default(),
        }),
    );
    let mut kernel = AuthoringKernel::with_schema(schema);
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: start speaker=hazel\n> line subject=warm tone=\n> line voice=\n",
            )],
            [],
        ))
        .expect("context fixture accepted");
    let document = key("main.recite");
    let QueryResult::Ready(tone) = kernel.snapshot().complete(&document, position(2, 26)) else {
        panic!("metadata-key context resolves");
    };
    assert_eq!(
        tone.iter()
            .map(|candidate| candidate.name())
            .collect::<Vec<_>>(),
        ["market"]
    );
    let QueryResult::Ready(voice) = kernel.snapshot().complete(&document, position(3, 14)) else {
        panic!("block default speaker context resolves");
    };
    assert_eq!(
        voice
            .iter()
            .map(|candidate| candidate.name())
            .collect::<Vec<_>>(),
        ["soft"]
    );
}

#[test]
fn contextual_metadata_rejects_duplicate_and_wrong_target_context() {
    let mut schema = ProjectSchema::empty_v1();
    schema.metadata.insert(
        "tone".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("tone_by_subject".to_owned()),
        },
    );
    schema.metadata_domains.insert(
        "tone_by_subject".to_owned(),
        MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
            selector: MetadataContextSelector::MetadataKey("subject".to_owned()),
            values_by_context: BTreeSet::from([(
                "warm".to_owned(),
                BTreeSet::from(["market".to_owned()]),
            )])
            .into_iter()
            .collect(),
            missing_context: MissingMetadataContextPolicy::Empty,
            provenance: Default::default(),
        }),
    );
    let mut kernel = AuthoringKernel::with_schema(schema);
    kernel
        .apply(AuthoringRequest::new(
            SnapshotGeneration::initial(),
            [SavedDocument::new(
                key("main.recite"),
                ":: start\n> line subject=warm subject=warm tone=\n",
            )],
            [],
        ))
        .expect("context fixture accepted");
    let result = kernel
        .snapshot()
        .complete(&key("main.recite"), position(2, 48));
    assert!(matches!(
        result,
        QueryResult::Partial { .. } | QueryResult::Unavailable { .. }
    ));
}
