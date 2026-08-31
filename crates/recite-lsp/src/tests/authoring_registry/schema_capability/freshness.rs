pub(super) fn assert_absent_content_fingerprint(catalog: &recite_ui::UiCatalog) {
    let producer =
        recite_core::ProducerIdentity::new("adapter", "generated").expect("producer identity");
    let fingerprint = recite_core::ProducerFingerprint {
        id: "items".to_owned(),
        kind: "fixture".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "items-v1".to_owned(),
    };
    let mut schema = recite_core::ProjectSchema::empty_v1();
    schema.producer_metadata = Some(recite_core::ProducerMetadata {
        producer: Some(producer),
        content_fingerprint: None,
        schema_export_version: None,
        inclusion_policy: None,
        producer_fingerprints: vec![fingerprint.clone()],
    });
    let summary = recite_compiler::SchemaSummary::from_schema(&schema);
    let detail =
        crate::features::schema_hover::hover_detail(None, &summary, &[fingerprint], &catalog);

    assert!(detail.contains("ABSENT-CONTENT"));
    assert!(detail.contains("items-v1"));
}
