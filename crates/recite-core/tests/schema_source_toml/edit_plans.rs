use std::collections::BTreeSet;

use recite_core::{
    AvailabilityReasonDefinition, ConditionDefinition, ConditionReturnType, EffectDefinition,
    EffectMode, ParameterDefinition, SchemaSourceEdit, SchemaSourceEditError, SchemaTypeRef,
    load_schema_source_str,
};

fn source(text: &str) -> recite_core::SchemaSource {
    match load_schema_source_str("schema.toml", text).source {
        Some(source) => source,
        None => panic!("valid standalone schema required"),
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
