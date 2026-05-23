use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    EnumTypeDefinition, MetadataDefinition, MetadataTarget, ProjectSchema, RegistryDefinition,
    SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
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
                type_ref: SchemaTypeRef::String,
                repeatable: false,
            },
        ),
        (
            "flag".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Bool,
                repeatable: false,
            },
        ),
        (
            "mood".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Enum("mood_kind".to_owned()),
                repeatable: false,
            },
        ),
        (
            "portrait".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::String,
                repeatable: false,
            },
        ),
        (
            "priority".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Int,
                repeatable: false,
            },
        ),
        (
            "sfx".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Choice, MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Registry("sound".to_owned()),
                repeatable: true,
            },
        ),
        (
            "talker".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Speaker,
                repeatable: false,
            },
        ),
        (
            "weight".to_owned(),
            MetadataDefinition {
                targets: BTreeSet::from([MetadataTarget::Line]),
                type_ref: SchemaTypeRef::Float,
                repeatable: false,
            },
        ),
    ]);
    schema
}

#[test]
fn accepts_schema_declared_metadata_on_supported_targets() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default block_tag=room\n",
            "> intro speaker=hazel portrait=neutral mood=calm priority=3 weight=1.5 flag=true talker=rhea sfx=snap sfx=door_close\n",
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
        ":: start default portrait=neutral\n> intro speaker=hazel\n  Hello.\n",
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
        ":: start default\n> intro speaker=hazel portrait=neutral portrait=flat\n  Hello.\n",
    )];

    let report = validate_source_files_with_schema(&files, &schema);

    assert_codes(&report, ["RECITE_VALIDATE028"]);
    assert_spans(&report, [(2, 40)]);
}

#[test]
fn reports_scalar_metadata_type_mismatches_on_value_spans() {
    let schema = metadata_schema();
    let files = vec![lower(
        "dialogue/start.recite",
        concat!(
            ":: start default\n",
            "> intro speaker=hazel priority=\"high\" weight=heavy flag=yes portrait=[flat]\n",
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
        ],
    );
    assert_spans(&report, [(2, 32), (2, 46), (2, 57), (2, 70)]);
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
