use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    ContextualMetadataDomain, EnumTypeDefinition, FlatMetadataDomain, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy,
    ProjectSchema, RegistryDefinition, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
};

pub(super) fn metadata_schema() -> ProjectSchema {
    let mut schema = ProjectSchema::empty_v1();
    schema.types = BTreeMap::from([(
        "mood_kind".to_owned(),
        SchemaTypeDefinition::Enum(EnumTypeDefinition {
            values: BTreeSet::from(["alert".to_owned(), "calm".to_owned()]),
        }),
    )]);
    schema.registries = BTreeMap::from([(
        "sound".to_owned(),
        RegistryDefinition {
            values: BTreeSet::from(["door_close".to_owned(), "snap".to_owned()]),
            origin: None,
            ..Default::default()
        },
    )]);
    schema.speakers = BTreeMap::from([
        ("hazel".to_owned(), SpeakerDefinition { display_name: None }),
        ("rhea".to_owned(), SpeakerDefinition { display_name: None }),
    ]);
    schema.metadata = BTreeMap::from([
        (
            "block_tag".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Block]),
                type_ref: SchemaTypeRef::Symbol,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "caption".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::String,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "flag".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Bool,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "mood".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Enum("mood_kind".to_owned()),
                repeatable: false,
                domain: None,
            },
        ),
        (
            "portrait".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::String,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "priority".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Int,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "sfx".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Choice, MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Registry("sound".to_owned()),
                repeatable: true,
                domain: None,
            },
        ),
        (
            "route".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Symbol,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "talker".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Speaker,
                repeatable: false,
                domain: None,
            },
        ),
        (
            "weight".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Float,
                repeatable: false,
                domain: None,
            },
        ),
    ]);
    schema
}

pub(super) fn metadata_domain_schema() -> ProjectSchema {
    let mut schema = metadata_schema();
    schema.metadata_domains = BTreeMap::from([
        (
            "portrait_all".to_owned(),
            MetadataDomainDefinition::Flat(FlatMetadataDomain {
                values: BTreeSet::from(["flat".to_owned(), "neutral".to_owned()]),
                ..Default::default()
            }),
        ),
        (
            "portrait_by_speaker".to_owned(),
            MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                selector: MetadataContextSelector::FieldSpeaker,
                values_by_context: BTreeMap::from([
                    (
                        "hazel".to_owned(),
                        BTreeSet::from(["flat".to_owned(), "neutral".to_owned()]),
                    ),
                    ("rhea".to_owned(), BTreeSet::from(["flat".to_owned()])),
                ]),
                missing_context: MissingMetadataContextPolicy::Fallback {
                    domain: "portrait_all".to_owned(),
                },
                provenance: Default::default(),
            }),
        ),
        (
            "emotion_by_subject".to_owned(),
            MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                selector: MetadataContextSelector::MetadataKey("subject".to_owned()),
                values_by_context: BTreeMap::from([
                    (
                        "hazel".to_owned(),
                        BTreeSet::from(["guarded".to_owned(), "wry".to_owned()]),
                    ),
                    (
                        "rhea".to_owned(),
                        BTreeSet::from(["angry".to_owned(), "calm".to_owned()]),
                    ),
                ]),
                missing_context: MissingMetadataContextPolicy::Diagnostic,
                provenance: Default::default(),
            }),
        ),
        (
            "portrait_by_speaker_diagnostic".to_owned(),
            MetadataDomainDefinition::Contextual(ContextualMetadataDomain {
                selector: MetadataContextSelector::FieldSpeaker,
                values_by_context: BTreeMap::from([(
                    "hazel".to_owned(),
                    BTreeSet::from(["flat".to_owned()]),
                )]),
                missing_context: MissingMetadataContextPolicy::Diagnostic,
                provenance: Default::default(),
            }),
        ),
    ]);
    schema.metadata.insert(
        "portrait_domain".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("portrait_by_speaker".to_owned()),
        },
    );
    schema.metadata.insert(
        "emotion".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("emotion_by_subject".to_owned()),
        },
    );
    schema.metadata.insert(
        "subject".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: true,
            domain: None,
        },
    );
    schema.metadata.insert(
        "block_context".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Block]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("portrait_by_speaker_diagnostic".to_owned()),
        },
    );
    schema.metadata.insert(
        "tags".to_owned(),
        MetadataDefinition {
            targets: BTreeSet::from([MetadataTarget::Line]),
            type_ref: SchemaTypeRef::Symbol,
            repeatable: false,
            domain: Some("portrait_all".to_owned()),
        },
    );
    schema
}
