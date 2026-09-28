use recite_core::schema::{
    ProducerIdentity, ProjectSchema, SpeakerDefinition, export_schema_manifest_json,
    export_schema_manifest_json_with_producer, load_schema_manifest_str, load_schema_source_str,
};

#[test]
fn exported_host_schema_is_validated_by_manifest_loader() {
    let schema = ProjectSchema::empty_v1();
    let json = export_schema_manifest_json(&schema).expect("valid host schema");
    let loaded = load_schema_manifest_str("host.json", &json);
    assert!(loaded.diagnostics.is_empty());
    assert_eq!(loaded.schema, Some(schema));

    let mut invalid = ProjectSchema::empty_v1();
    invalid.schema_version = 0;
    assert!(
        !export_schema_manifest_json(&invalid)
            .expect_err("invalid host schema must be rejected")
            .is_empty()
    );
}

#[test]
fn native_producer_fingerprint_uses_identity_and_canonical_content() {
    let source = include_str!("../../../fixtures/schema/valid/standalone.toml");
    let first = load_schema_source_str("one/schema.toml", source)
        .source
        .expect("source");
    let renamed = load_schema_source_str("renamed/schema.toml", source)
        .source
        .expect("source");
    let producer = ProducerIdentity::new("unity", "registration-guid").expect("identity");
    let export = |schema: &ProjectSchema, producer: ProducerIdentity| {
        let json =
            export_schema_manifest_json_with_producer(schema, producer).expect("native export");
        load_schema_manifest_str("generated.json", &json)
            .schema
            .expect("generated schema")
    };
    let one = export(first.schema(), producer.clone());
    let same = export(renamed.schema(), producer);
    assert_eq!(
        one, same,
        "path rename cannot alter native producer identity"
    );
    let metadata = one.producer_metadata.as_ref().expect("producer metadata");
    assert_eq!(
        metadata.producer.as_ref().expect("producer").kind(),
        "unity"
    );
    assert_eq!(metadata.producer_fingerprints.len(), 1);
    assert_eq!(metadata.producer_fingerprints[0].kind, "unity");
    assert_eq!(metadata.producer_fingerprints[0].id, "registration-guid");
    let mut changed = first.schema().clone();
    changed.speakers.insert(
        "new_speaker".to_owned(),
        SpeakerDefinition { display_name: None },
    );
    let changed = export(
        &changed,
        ProducerIdentity::new("unity", "registration-guid").expect("identity"),
    );
    assert_ne!(
        metadata.producer_fingerprints,
        changed
            .producer_metadata
            .expect("metadata")
            .producer_fingerprints
    );
    let godot = export(
        first.schema(),
        ProducerIdentity::new("godot", "registration-guid").expect("identity"),
    );
    assert_ne!(
        metadata.producer_fingerprints,
        godot
            .producer_metadata
            .expect("metadata")
            .producer_fingerprints
    );
}
