use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    AvailabilityReasonArgBinding, AvailabilityReasonDefinition, AvailabilityReasonId,
    ConditionAvailabilityReasonMapping, ConditionDefinition, ConditionReturnType, EffectDefinition,
    EffectMode, ParameterDefinition, ProducerMetadataValue, ProducerOrigin, SchemaLiteralValue,
    SchemaSourceEdit, SchemaSourceEditError, SchemaTypeRef, load_schema_source_str,
};

fn source(text: &str) -> recite_core::SchemaSource {
    match load_schema_source_str("schema.toml", text).source {
        Some(source) => source,
        None => panic!("valid standalone schema required"),
    }
}

fn reason_id(name: &str) -> AvailabilityReasonId {
    match AvailabilityReasonId::new(name) {
        Ok(id) => id,
        Err(error) => panic!("test reason id must be valid: {error}"),
    }
}

#[test]
fn plan_is_non_mutating_and_applies_only_to_matching_exact_source() {
    let text = "schema_version = 1\n[producer] # owner\nid = \"dialogue\" # identity\n";
    let original = source(text);
    let mut current = original.clone();
    let plan = original
        .plan_edit(SchemaSourceEdit::SetProducerId("dialogue-v2".to_owned()))
        .expect("plan");

    assert_eq!(original.source_text(), text);
    assert!(plan.replacement_text().contains("# owner"));
    assert!(plan.replacement_text().contains("# identity"));
    plan.apply(&mut current).expect("matching source applies");
    assert!(current.source_text().contains("id = \"dialogue-v2\""));

    let mut stale = source(&text.replace("# owner", "# changed"));
    assert!(matches!(
        plan.apply(&mut stale),
        Err(SchemaSourceEditError::StaleSource { .. })
    ));
}

#[test]
fn plan_rejects_semantically_stale_source() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    let plan = original
        .plan_edit(SchemaSourceEdit::SetProducerId("dialogue-v2".to_owned()))
        .expect("plan");
    let mut stale = source("schema_version = 1\n[producer]\nid = \"other\"\n");
    assert!(matches!(
        plan.apply(&mut stale),
        Err(SchemaSourceEditError::StaleSource { .. })
    ));
}

#[test]
fn typed_declaration_plans_round_trip_exact_canonical_definitions() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    let condition = ConditionDefinition {
        params: vec![ParameterDefinition {
            name: "actor".to_owned(),
            type_ref: SchemaTypeRef::Speaker,
        }],
        returns: ConditionReturnType::Bool,
        availability_reason: None,
    };
    let effect = EffectDefinition {
        modes: BTreeSet::from([EffectMode::Blocking, EffectMode::Deferred]),
        params: vec![ParameterDefinition {
            name: "amount".to_owned(),
            type_ref: SchemaTypeRef::Int,
        }],
    };
    let reason = AvailabilityReasonDefinition {
        template: "Not ready".to_owned(),
        params: Vec::new(),
        origin: None,
    };
    let mut edited = original.clone();
    original
        .plan_edit(SchemaSourceEdit::AddAvailabilityReason {
            name: "not_ready".to_owned(),
            definition: reason,
        })
        .expect("reason plan")
        .apply(&mut edited)
        .expect("reason apply");
    edited
        .plan_edit(SchemaSourceEdit::AddCondition {
            name: "ready".to_owned(),
            definition: condition.clone(),
        })
        .expect("condition plan")
        .apply(&mut edited)
        .expect("condition apply");
    edited
        .plan_edit(SchemaSourceEdit::AddEffect {
            name: "spend".to_owned(),
            definition: effect.clone(),
        })
        .expect("effect plan")
        .apply(&mut edited)
        .expect("effect apply");

    assert_eq!(edited.schema().conditions["ready"], condition);
    assert_eq!(edited.schema().effects["spend"], effect);
    assert!(
        edited
            .source_text()
            .contains("modes = [\"deferred\", \"blocking\"]")
    );
}

#[test]
fn malformed_typed_declaration_retains_core_diagnostic_authority() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    let error = original.plan_edit(SchemaSourceEdit::AddCondition {
        name: "bad.name".to_owned(),
        definition: ConditionDefinition {
            params: Vec::new(),
            returns: ConditionReturnType::Enum("missing".to_owned()),
            availability_reason: None,
        },
    });
    assert!(matches!(error, Err(SchemaSourceEditError::Diagnostics(_))));
}

#[test]
fn float_literal_lexeme_and_reason_origin_round_trip_without_loss() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    let origin = ProducerOrigin {
        kind: "data_table".to_owned(),
        id: "content/reasons.csv".to_owned(),
        label: Some("Reasons".to_owned()),
        extensions: BTreeMap::from([(
            "x-recite:source".to_owned(),
            ProducerMetadataValue::Object(BTreeMap::from([
                (
                    "precise".to_owned(),
                    ProducerMetadataValue::Number("1.23456789012345678901234567890".to_owned()),
                ),
                (
                    "nested".to_owned(),
                    ProducerMetadataValue::Object(BTreeMap::from([(
                        "exponent".to_owned(),
                        ProducerMetadataValue::Number("1e+2".to_owned()),
                    )])),
                ),
                (
                    "integer".to_owned(),
                    ProducerMetadataValue::Number("1234567890123456789".to_owned()),
                ),
                (
                    "array".to_owned(),
                    ProducerMetadataValue::Array(vec![
                        ProducerMetadataValue::Number("0.25".to_owned()),
                        ProducerMetadataValue::Number("2e-3".to_owned()),
                    ]),
                ),
            ])),
        )]),
    };
    let reason = AvailabilityReasonDefinition {
        template: "Value is {value}".to_owned(),
        params: vec![ParameterDefinition {
            name: "value".to_owned(),
            type_ref: SchemaTypeRef::Float,
        }],
        origin: Some(origin.clone()),
    };
    let exact = "1.23456789012345678901234567890".to_owned();
    let condition = ConditionDefinition {
        params: Vec::new(),
        returns: ConditionReturnType::Bool,
        availability_reason: Some(ConditionAvailabilityReasonMapping {
            reason: reason_id("not_ready"),
            args: BTreeMap::from([(
                "value".to_owned(),
                AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(exact.clone())),
            )]),
        }),
    };
    let mut edited = original.clone();
    original
        .plan_edit(SchemaSourceEdit::AddAvailabilityReason {
            name: "not_ready".to_owned(),
            definition: reason,
        })
        .expect("reason plan")
        .apply(&mut edited)
        .expect("reason apply");
    edited
        .plan_edit(SchemaSourceEdit::AddCondition {
            name: "ready".to_owned(),
            definition: condition.clone(),
        })
        .expect("condition plan")
        .apply(&mut edited)
        .expect("condition apply");

    assert_eq!(edited.schema().conditions["ready"], condition);
    assert_eq!(
        edited.schema().availability_reasons[&reason_id("not_ready")].origin,
        Some(origin)
    );
    assert!(edited.source_text().contains(&exact));
    assert!(edited.source_text().contains("1e+2"));
    assert!(edited.source_text().contains("1234567890123456789"));
    let round_trip = source(&edited.source_text());
    assert_eq!(edited.schema_fingerprint(), round_trip.schema_fingerprint());
    assert_eq!(edited.source_fingerprint(), round_trip.source_fingerprint());
}

#[test]
fn origin_number_edits_reject_non_json_and_toml_only_lexemes_recursively() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    let hostile = [
        ProducerMetadataValue::Number("true".to_owned()),
        ProducerMetadataValue::Number("\"quoted\"".to_owned()),
        ProducerMetadataValue::Number("[1, 2]".to_owned()),
        ProducerMetadataValue::Number("1970-01-01T00:00:00Z".to_owned()),
        ProducerMetadataValue::Number("+1".to_owned()),
        ProducerMetadataValue::Number("1_000".to_owned()),
        ProducerMetadataValue::Number("0x10".to_owned()),
        ProducerMetadataValue::Number("-0".to_owned()),
        ProducerMetadataValue::Number("0e0".to_owned()),
        ProducerMetadataValue::Number("0E+00".to_owned()),
        ProducerMetadataValue::Number("1E+2".to_owned()),
        ProducerMetadataValue::Number("1E-002".to_owned()),
        ProducerMetadataValue::Number("1e002".to_owned()),
        ProducerMetadataValue::Array(vec![ProducerMetadataValue::Number("1_000".to_owned())]),
        ProducerMetadataValue::Object(BTreeMap::from([(
            "nested".to_owned(),
            ProducerMetadataValue::Number("true".to_owned()),
        )])),
    ];
    for value in hostile {
        let error = original.plan_edit(SchemaSourceEdit::AddAvailabilityReason {
            name: "blocked".to_owned(),
            definition: AvailabilityReasonDefinition {
                template: "Blocked".to_owned(),
                params: Vec::new(),
                origin: Some(ProducerOrigin {
                    kind: "data_table".to_owned(),
                    id: "content/reasons.csv".to_owned(),
                    label: None,
                    extensions: BTreeMap::from([("x-recite:value".to_owned(), value)]),
                }),
            },
        });
        assert!(matches!(
            error,
            Err(SchemaSourceEditError::InvalidArgument(message))
                if message.contains("canonical JSON number lexeme")
        ));
    }
}

#[test]
fn origin_extensions_reject_reserved_and_unnamespaced_keys() {
    let original = source("schema_version = 1\n[producer]\nid = \"dialogue\"\n");
    for key in ["kind", "id", "label", "source"] {
        let error = original.plan_edit(SchemaSourceEdit::AddAvailabilityReason {
            name: "blocked".to_owned(),
            definition: AvailabilityReasonDefinition {
                template: "Blocked".to_owned(),
                params: Vec::new(),
                origin: Some(ProducerOrigin {
                    kind: "data_table".to_owned(),
                    id: "content/reasons.csv".to_owned(),
                    label: None,
                    extensions: BTreeMap::from([(
                        key.to_owned(),
                        ProducerMetadataValue::String("value".to_owned()),
                    )]),
                }),
            },
        });
        assert!(matches!(
            error,
            Err(SchemaSourceEditError::InvalidArgument(message))
                if message.contains(key)
        ));
    }
}

#[test]
fn declaration_plans_accept_loader_valid_empty_inline_sections() {
    let original = source(
        "schema_version = 1\nconditions = {} # conditions\neffects = {} # effects\navailability_reasons = {} # reasons\n[producer]\nid = \"dialogue\"\n",
    );
    let mut edited = original.clone();
    edited
        .plan_edit(SchemaSourceEdit::AddCondition {
            name: "ready".to_owned(),
            definition: ConditionDefinition {
                params: Vec::new(),
                returns: ConditionReturnType::Bool,
                availability_reason: None,
            },
        })
        .expect("condition plan from inline section")
        .apply(&mut edited)
        .expect("condition apply");
    edited
        .plan_edit(SchemaSourceEdit::AddEffect {
            name: "spend".to_owned(),
            definition: EffectDefinition {
                modes: BTreeSet::from([EffectMode::Immediate]),
                params: Vec::new(),
            },
        })
        .expect("effect plan from inline section")
        .apply(&mut edited)
        .expect("effect apply");
    edited
        .plan_edit(SchemaSourceEdit::AddAvailabilityReason {
            name: "blocked".to_owned(),
            definition: AvailabilityReasonDefinition {
                template: "Blocked".to_owned(),
                params: Vec::new(),
                origin: None,
            },
        })
        .expect("reason plan from inline section")
        .apply(&mut edited)
        .expect("reason apply");

    assert!(edited.schema().conditions.contains_key("ready"));
    assert!(edited.schema().effects.contains_key("spend"));
    assert!(
        edited
            .schema()
            .availability_reasons
            .contains_key(&reason_id("blocked"))
    );
    assert!(edited.source_text().contains("# conditions"));
    assert!(edited.source_text().contains("# effects"));
    assert!(edited.source_text().contains("# reasons"));
}
