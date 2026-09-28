use std::collections::BTreeSet;

use recite_bevy::{ReciteEnum, ReciteSchema, ReciteType, recite_schema};
use recite_core::schema::{
    ConditionReturnType, EnumTypeDefinition, ProducerIdentity, SchemaTypeDefinition, SchemaTypeRef,
    load_schema_manifest_str,
};

struct ActorId;
impl ReciteType for ActorId {
    fn schema_type() -> SchemaTypeRef {
        SchemaTypeRef::String
    }
}

struct Stage;
impl ReciteEnum for Stage {
    const NAME: &'static str = "stage";
}

#[test]
fn typed_builder_and_macro_share_canonical_export() {
    let mut schema = ReciteSchema::new();
    schema.schema_mut().types.insert(
        "stage".to_owned(),
        SchemaTypeDefinition::Enum(EnumTypeDefinition {
            values: BTreeSet::from(["opening".to_owned(), "ending".to_owned()]),
        }),
    );
    schema
        .condition("trust_gte")
        .expect("unique")
        .param::<ActorId>("actor_a")
        .param::<i32>("threshold")
        .returns_bool();
    schema
        .condition("thread_stage")
        .expect("unique")
        .param::<String>("thread_id")
        .returns_enum::<Stage>();
    let _ = schema
        .effect("play_sfx")
        .expect("unique")
        .immediate()
        .param::<String>("sound_effect");
    recite_schema!(schema;
        condition can_enter(actor: ActorId, minimum: i64) -> bool;
        effect grant_key(actor: ActorId) [blocking];
        effect mark_seen() [deferred];
    )
    .expect("unique macro declarations");
    let json = schema.export_json().expect("canonical export");
    let report = load_schema_manifest_str("generated.json", &json);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let lowered = report.schema.expect("lowered schema");
    assert_eq!(lowered, *schema.schema());
    assert_eq!(
        lowered.conditions["thread_stage"].returns,
        ConditionReturnType::Enum("stage".to_owned())
    );
    assert_eq!(
        lowered.conditions["trust_gte"].params[1].type_ref,
        SchemaTypeRef::Int
    );
    assert_eq!(lowered.effects["play_sfx"].params[0].name, "sound_effect");
    assert_eq!(lowered.effects["grant_key"].modes.len(), 1);
    assert_eq!(lowered.effects["mark_seen"].modes.len(), 1);
    let native = schema
        .export_json_with_producer(
            ProducerIdentity::new("bevy", "registration/scene").expect("identity"),
        )
        .expect("native export");
    assert_eq!(
        load_schema_manifest_str("native.json", &native)
            .schema
            .expect("native schema")
            .producer_metadata
            .expect("metadata")
            .producer
            .expect("producer")
            .kind(),
        "bevy"
    );
}

#[test]
fn duplicate_registration_preserves_prior_declaration() {
    let mut schema = ReciteSchema::new();
    schema
        .condition("owned")
        .expect("first")
        .param::<i32>("first")
        .returns_bool();
    let error = schema.condition("owned").err().expect("duplicate");
    assert_eq!(error.kind, "condition");
    assert_eq!(error.name, "owned");
    assert_eq!(schema.schema().conditions["owned"].params[0].name, "first");
    let _ = schema.effect("effect").expect("first").blocking();
    assert_eq!(
        schema.effect("effect").err().expect("duplicate").kind,
        "effect"
    );
    let macro_error = recite_schema!(schema;
        condition owned() -> bool;
    )
    .expect_err("macro duplicate");
    assert_eq!(macro_error.name, "owned");
}
