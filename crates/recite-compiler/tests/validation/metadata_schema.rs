use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    ContextualMetadataDomain, EnumTypeDefinition, FlatMetadataDomain, MetadataContextSelector,
    MetadataDefinition, MetadataDomainDefinition, MetadataTarget, MissingMetadataContextPolicy,
    ProjectSchema, RegistryDefinition, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
};

use super::*;

fn metadata_schema() -> ProjectSchema {
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

fn metadata_domain_schema() -> ProjectSchema {
    let mut schema = metadata_schema();
    schema.metadata_domains = BTreeMap::from([
        (
            "portrait_all".to_owned(),
            MetadataDomainDefinition::Flat(FlatMetadataDomain {
                values: BTreeSet::from(["flat".to_owned(), "neutral".to_owned()]),
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

#[test]
fn accepts_schema_declared_metadata_on_supported_targets() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=room\n",
            "> intro speaker=hazel portrait=\"neutral\" caption=\"Hello.\" mood=calm priority=3 weight=1.5 flag=true route=north talker=rhea sfx=snap sfx=door_close\n",
            "  Hello.\n",
            "  ? ask sfx=snap\n",
            "    Ask.\n",
            "    -> END\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "valid metadata should pass: {report:?}");
}

#[test]
fn validates_flat_and_contextual_metadata_domains() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=hazel\n",
            "> intro portrait_domain=neutral subject=rhea emotion=calm tags=[flat, neutral]\n",
            "  Hello.\n",
            "> explicit speaker=rhea portrait_domain=flat\n",
            "  Hello.\n",
            "> fallback portrait_domain=neutral\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(
        report.is_ok(),
        "valid metadata domains should pass: {report:?}"
    );
}

#[test]
fn reports_invalid_metadata_domain_values_on_value_spans() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default speaker=rhea\n",
            "> intro portrait_domain=neutral tags=[flat, missing]\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE031", "RECITE_VALIDATE031"]);
    assert_spans(&report, [(2, 25), (2, 38)]);
}

#[test]
fn reports_missing_and_malformed_metadata_domain_context() {
    let schema = metadata_domain_schema();
    let missing = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&missing, &schema);
    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(2, 17)]);

    let malformed = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro subject=\"rhea\" emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&malformed, &schema);
    assert_codes(&report, ["RECITE_VALIDATE029", "RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 17), (2, 17)]);
}

#[test]
fn block_metadata_does_not_use_default_speaker_as_field_speaker_context() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default speaker=hazel block_context=flat\n> intro\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE032"]);
    assert_spans(&report, [(1, 46)]);
}

#[test]
fn repeated_metadata_selector_reports_selector_span() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro subject=rhea subject=hazel emotion=calm\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE033"]);
    assert_spans(&report, [(2, 22)]);
}

#[test]
fn reports_mixed_array_metadata_type_mismatch_before_domain_validation() {
    let schema = metadata_domain_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro tags=[flat, \"neutral\"]\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE029"]);
    assert_spans(&report, [(2, 14)]);
}

#[test]
fn reports_unknown_metadata_key_on_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro speaker=hazel mystery=flat\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE026"]);
    assert_spans(&report, [(2, 23)]);
}

#[test]
fn reports_invalid_metadata_target_on_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default portrait=\"neutral\"\n> intro speaker=hazel\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE027"]);
    assert_spans(&report, [(1, 18)]);
}

#[test]
fn reports_non_repeatable_duplicate_metadata_on_duplicate_key_span() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        ":: start default\n> intro speaker=hazel portrait=\"neutral\" portrait=\"flat\"\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE028"]);
    assert_spans(&report, [(2, 42)]);
}

#[test]
fn reports_scalar_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro speaker=hazel priority=\"high\" weight=heavy flag=yes portrait=[flat] caption=plain\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
        ],
    );
    assert_spans(&report, [(2, 32), (2, 46), (2, 57), (2, 70), (2, 85)]);
}

#[test]
fn reports_quoted_reference_and_symbol_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=\"room\"\n",
            "> intro speaker=hazel talker=\"rhea\" mood=\"calm\" sfx=\"snap\" route=\"north\"\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
            "RECITE_VALIDATE029",
        ],
    );
    assert_spans(&report, [(1, 28), (2, 30), (2, 42), (2, 53), (2, 66)]);
}

#[test]
fn reports_invalid_speaker_enum_and_registry_metadata_values_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro speaker=hazel talker=ghost mood=angry sfx=missing\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(
        &report,
        [
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
            "RECITE_VALIDATE030",
        ],
    );
    assert_spans(&report, [(2, 30), (2, 41), (2, 51)]);
}

#[test]
fn skips_metadata_schema_validation_without_schema() {
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default portrait=neutral\n",
            "> intro speaker=hazel mystery=flat portrait=[flat]\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files(&files);

    assert!(
        report.is_ok(),
        "schema-less metadata should pass: {report:?}"
    );
}

#[test]
fn accepts_metadata_only_symbol_type_on_line_metadata() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro speaker=hazel route=north\n",
            "  Hello.\n",
        ),
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert!(report.is_ok(), "symbol metadata should pass: {report:?}");
}
