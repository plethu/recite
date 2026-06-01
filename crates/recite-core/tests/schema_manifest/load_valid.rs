use recite_core::{
    ConditionReturnType, EffectMode, MetadataTarget, SchemaTypeDefinition, SchemaTypeRef,
    load_schema_manifest_str,
};

use crate::diagnostic_codes;

#[test]
fn valid_generated_manifest_loads_into_canonical_schema() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );

    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid schema manifest");

    assert_eq!(schema.schema_version, 1);
    assert_eq!(
        schema.types.keys().map(String::as_str).collect::<Vec<_>>(),
        ["thread_stage_kind"]
    );
    let SchemaTypeDefinition::Enum(thread_stage_kind) = &schema.types["thread_stage_kind"];
    assert_eq!(
        thread_stage_kind
            .values
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["angry", "completed", "fine", "fresh", "tired"]
    );

    assert_eq!(
        schema.conditions["trust_gte"].returns,
        ConditionReturnType::Bool
    );
    assert_eq!(
        schema.conditions["thread_stage"].returns,
        ConditionReturnType::Enum("thread_stage_kind".to_owned())
    );
    assert_eq!(
        schema.effects["advance_thread"]
            .modes
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [EffectMode::Deferred, EffectMode::Blocking]
    );
    assert_eq!(
        schema.metadata["sfx"]
            .targets
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [MetadataTarget::Choice, MetadataTarget::Line]
    );
    assert!(!schema.markup["shake"].allows_nesting);
}

#[test]
fn manifest_type_refs_support_the_issue_52_surface_exactly() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/generated_manifest.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let schema = report.schema.expect("valid schema manifest");

    assert_eq!(
        schema.effects["advance_thread"].params[0].type_ref,
        SchemaTypeRef::Registry("thread".to_owned())
    );
    assert_eq!(
        schema.effects["advance_thread"].params[1].type_ref,
        SchemaTypeRef::Enum("thread_stage_kind".to_owned())
    );
    assert_eq!(
        schema.conditions["trust_gte"].params[0].type_ref,
        SchemaTypeRef::Speaker
    );
}

#[test]
fn metadata_type_refs_support_symbol() {
    let report = load_schema_manifest_str(
        "fixtures/schema/valid/symbol_metadata_type.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "route": {
      "targets": ["line"],
      "type": "symbol"
    }
  }
}"#,
    );
    assert_eq!(diagnostic_codes(&report), Vec::<&str>::new());
    let schema = report.schema.expect("valid symbol metadata schema");
    assert_eq!(schema.metadata["route"].type_ref, SchemaTypeRef::Symbol);
}
