use std::collections::BTreeMap;

use recite_compiler::pot::extract_pot_with_schema;
use recite_core::{
    AvailabilityReasonId,
    schema::{
        AvailabilityReasonDefinition, ConditionDefinition, ConditionReturnType, EnumTypeDefinition,
        ParameterDefinition, PresentationAffordanceOutputDefinition,
        PresentationLabelArgDefinition, PresentationLabelDefinition, ProducerOrigin, ProjectSchema,
        ProjectionInput, ProjectionInputRef, ProjectionOutputTarget, ProjectionQueryDefinition,
        ProjectionQueryFunctionDefinition, SchemaLiteralValue,
        SchemaPresentationProjectorDefinition, SchemaProjectionInputSource,
        SchemaProjectionSelector, SchemaTypeDefinition, SchemaTypeRef, SpeakerDefinition,
    },
};

use super::{fixture_support, project_inputs};

#[test]
fn extracts_lines_choices_and_speaker_display_names_to_pot() {
    let mut schema = ProjectSchema::empty_v1();
    schema.speakers = BTreeMap::from([
        (
            "narrator".to_owned(),
            SpeakerDefinition {
                display_name: Some("Narrator".to_owned()),
            },
        ),
        (
            "hazel".to_owned(),
            SpeakerDefinition {
                display_name: Some("Hazel".to_owned()),
            },
        ),
        (
            "silent".to_owned(),
            SpeakerDefinition { display_name: None },
        ),
    ]);
    add_project_input_conditions(&mut schema);

    let report = extract_pot_with_schema(project_inputs(), &schema);
    assert!(report.is_ok(), "{:?}", report.diagnostics);
    let pot = report.catalog.expect("valid inputs produce a POT catalog");

    fixture_support::assert_text_snapshot(
        &pot.to_pot_string(),
        "pot_extraction__lines_choices_and_speaker_display_names".to_owned(),
    );
}

#[test]
fn extracts_availability_reason_templates_to_pot_in_schema_order() {
    let mut schema = ProjectSchema::empty_v1();
    schema.availability_reasons = BTreeMap::from([
        (
            AvailabilityReasonId::new("z_reason").expect("valid reason id"),
            AvailabilityReasonDefinition {
                template: "Zed is blocked.".to_owned(),
                params: Vec::new(),
                origin: None,
            },
        ),
        (
            AvailabilityReasonId::new("trust_too_low").expect("valid reason id"),
            AvailabilityReasonDefinition {
                template: "{subject} does not trust {target} enough.".to_owned(),
                params: vec![
                    ParameterDefinition {
                        name: "subject".to_owned(),
                        type_ref: SchemaTypeRef::Speaker,
                    },
                    ParameterDefinition {
                        name: "target".to_owned(),
                        type_ref: SchemaTypeRef::Speaker,
                    },
                ],
                origin: Some(ProducerOrigin {
                    kind: "script_member".to_owned(),
                    id: "schema/reasons.rs".to_owned(),
                    label: None,
                    ..Default::default()
                }),
            },
        ),
    ]);
    add_project_input_conditions(&mut schema);

    let report = extract_pot_with_schema(project_inputs(), &schema);
    assert!(report.is_ok(), "{:?}", report.diagnostics);
    let pot = report.catalog.expect("valid inputs produce a POT catalog");
    let reason_entries = pot
        .entries
        .iter()
        .filter(|entry| entry.context.starts_with("availability_reason:"))
        .collect::<Vec<_>>();

    assert_eq!(
        reason_entries
            .iter()
            .map(|entry| entry.context.as_str())
            .collect::<Vec<_>>(),
        [
            "availability_reason:trust_too_low",
            "availability_reason:z_reason"
        ]
    );
    assert_eq!(
        reason_entries[0].source_text,
        "{subject} does not trust {target} enough."
    );
    assert_eq!(reason_entries[0].comments, ["availability reason template"]);
}

#[test]
fn extracts_presentation_label_templates_to_pot() {
    let mut schema = ProjectSchema::empty_v1();
    schema.projection_queries.insert(
        "actor_skill".to_owned(),
        ProjectionQueryFunctionDefinition {
            params: vec![ParameterDefinition {
                name: "skill".to_owned(),
                type_ref: SchemaTypeRef::String,
            }],
            returns: SchemaTypeRef::Int,
            max_calls_per_event: Some(1),
        },
    );
    schema.presentation_projectors = BTreeMap::from([(
        "choice_skill_prefix".to_owned(),
        SchemaPresentationProjectorDefinition {
            candidates: SchemaProjectionSelector::RuntimeEvent {
                kind: "prompt".to_owned(),
            },
            inputs: vec![
                ProjectionInput {
                    name: "skill".to_owned(),
                    source: SchemaProjectionInputSource::Literal(SchemaLiteralValue::String(
                        "Speech".to_owned(),
                    )),
                    type_ref: SchemaTypeRef::String,
                    required: true,
                },
                ProjectionInput {
                    name: "threshold".to_owned(),
                    source: SchemaProjectionInputSource::Literal(SchemaLiteralValue::Int(20)),
                    type_ref: SchemaTypeRef::Int,
                    required: true,
                },
            ],
            queries: BTreeMap::from([(
                "current".to_owned(),
                ProjectionQueryDefinition {
                    function: "actor_skill".to_owned(),
                    args: vec![ProjectionInputRef::Input {
                        name: "skill".to_owned(),
                    }],
                },
            )]),
            outputs: BTreeMap::from([(
                "prefix".to_owned(),
                PresentationAffordanceOutputDefinition {
                    target: ProjectionOutputTarget::Candidate,
                    kind: "badge".to_owned(),
                    slot: "prefix".to_owned(),
                    label: Some(PresentationLabelDefinition {
                        template_id: "skill_check_prefix".to_owned(),
                        source_text: "[{skill} {current}/{threshold}]".to_owned(),
                        args: BTreeMap::from([
                            (
                                "skill".to_owned(),
                                PresentationLabelArgDefinition {
                                    source: ProjectionInputRef::Input {
                                        name: "skill".to_owned(),
                                    },
                                    type_ref: SchemaTypeRef::String,
                                },
                            ),
                            (
                                "current".to_owned(),
                                PresentationLabelArgDefinition {
                                    source: ProjectionInputRef::QueryResult {
                                        name: "current".to_owned(),
                                    },
                                    type_ref: SchemaTypeRef::Int,
                                },
                            ),
                            (
                                "threshold".to_owned(),
                                PresentationLabelArgDefinition {
                                    source: ProjectionInputRef::Input {
                                        name: "threshold".to_owned(),
                                    },
                                    type_ref: SchemaTypeRef::Int,
                                },
                            ),
                        ]),
                    }),
                    fields: BTreeMap::new(),
                },
            )]),
        },
    )]);
    add_project_input_conditions(&mut schema);

    let report = extract_pot_with_schema(project_inputs(), &schema);
    assert!(report.is_ok(), "{:?}", report.diagnostics);
    let pot = report.catalog.expect("valid inputs produce a POT catalog");
    let label_entries = pot
        .entries
        .iter()
        .filter(|entry| entry.context.starts_with("presentation_label:"))
        .collect::<Vec<_>>();

    assert_eq!(label_entries.len(), 1);
    assert_eq!(
        label_entries[0].context,
        "presentation_label:skill_check_prefix"
    );
    assert_eq!(
        label_entries[0].source_text,
        "[{skill} {current}/{threshold}]"
    );
    assert_eq!(
        label_entries[0].comments,
        ["presentation label template: prefix"]
    );
}

fn add_project_input_conditions(schema: &mut ProjectSchema) {
    for speaker in ["narrator", "hazel"] {
        schema
            .speakers
            .entry(speaker.to_owned())
            .or_insert(SpeakerDefinition { display_name: None });
    }
    for (name, value) in [("actor_kind", "player"), ("thread_kind", "thread")] {
        schema.types.insert(
            name.to_owned(),
            SchemaTypeDefinition::Enum(EnumTypeDefinition {
                values: [value.to_owned()].into(),
            }),
        );
    }
    schema.types.insert(
        "stage_kind".to_owned(),
        SchemaTypeDefinition::Enum(EnumTypeDefinition {
            values: ["ready".to_owned(), "waiting".to_owned()].into(),
        }),
    );
    schema.conditions.insert(
        "trusts".to_owned(),
        ConditionDefinition {
            params: vec![ParameterDefinition {
                name: "actor".to_owned(),
                type_ref: SchemaTypeRef::Enum("actor_kind".to_owned()),
            }],
            returns: ConditionReturnType::Bool,
            availability_reason: None,
        },
    );
    schema.conditions.insert(
        "stage".to_owned(),
        ConditionDefinition {
            params: vec![ParameterDefinition {
                name: "thread".to_owned(),
                type_ref: SchemaTypeRef::Enum("thread_kind".to_owned()),
            }],
            returns: ConditionReturnType::Enum("stage_kind".to_owned()),
            availability_reason: None,
        },
    );
}
