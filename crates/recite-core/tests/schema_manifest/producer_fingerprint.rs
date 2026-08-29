use recite_core::{
    ContentFingerprintFreshness, MetadataDomainDefinition, ProducerFingerprint, ProducerFreshness,
    ProducerMetadata, ProducerOrigin, canonical_schema_fingerprint, compare_producer_fingerprints,
    compare_schema_producer_freshness, compare_schema_producer_freshness_detailed,
    load_schema_manifest_str,
};

fn full_manifest_schema() -> recite_core::ProjectSchema {
    load_schema_manifest_str(
        "fixtures/schema/valid/full_manifest.json",
        include_str!("../../../../fixtures/schema/valid/full_manifest.json"),
    )
    .schema
    .expect("full manifest fixture is valid")
}

fn fingerprint(id: &str, value: &str) -> ProducerFingerprint {
    ProducerFingerprint {
        id: id.to_owned(),
        kind: "asset".to_owned(),
        algorithm: "blake3".to_owned(),
        value: value.to_owned(),
    }
}

#[test]
fn producer_content_freshness_is_typed_deterministic_and_non_semantic() {
    let expected = vec![fingerprint("a", "1"), fingerprint("b", "2")];
    assert_eq!(
        compare_producer_fingerprints(&expected, &expected),
        ProducerFreshness::Fresh
    );
    assert!(matches!(
        compare_producer_fingerprints(&expected, &[fingerprint("a", "changed")]),
        ProducerFreshness::Mixed { .. }
    ));
    assert!(matches!(
        compare_producer_fingerprints(&expected, &[fingerprint("a", "1")]),
        ProducerFreshness::Missing { .. }
    ));
    assert!(matches!(
        compare_producer_fingerprints(&expected, &[fingerprint("a", "1"), fingerprint("c", "3")]),
        ProducerFreshness::Mixed { .. }
    ));
    let duplicate = [fingerprint("a", "1"), fingerprint("a", "changed")];
    assert!(matches!(
        compare_producer_fingerprints(&duplicate, &expected),
        ProducerFreshness::Invalid { .. }
    ));
    assert!(matches!(
        compare_producer_fingerprints(&expected, &duplicate),
        ProducerFreshness::Invalid { .. }
    ));
}

#[test]
fn producer_metadata_does_not_change_semantic_schema_fingerprint() {
    let base = full_manifest_schema();
    let fingerprint = canonical_schema_fingerprint(&base);

    let mut changed = base.clone();
    changed.producer_metadata = Some(ProducerMetadata {
        producer: Some(
            recite_core::ProducerIdentity::new("changed", "producer")
                .expect("valid producer identity"),
        ),
        content_fingerprint: Some(
            recite_core::producer_content_fingerprint(
                "blake3",
                "1111111111111111111111111111111111111111111111111111111111111111",
            )
            .expect("valid content fingerprint"),
        ),
        schema_export_version: Some(2),
        inclusion_policy: Some("changed-policy".to_owned()),
        producer_fingerprints: vec![ProducerFingerprint {
            id: "content/items".to_owned(),
            kind: "directory".to_owned(),
            algorithm: "blake3".to_owned(),
            value: "changed".to_owned(),
        }],
    });
    assert_eq!(fingerprint, canonical_schema_fingerprint(&changed));
}

#[test]
fn diagnostic_provenance_does_not_change_semantic_schema_fingerprint() {
    let base = full_manifest_schema();
    let fingerprint = canonical_schema_fingerprint(&base);

    let mut registry = base.clone();
    let registry_definition = registry
        .registries
        .get_mut("item")
        .expect("registry exists");
    registry_definition.origin = Some(ProducerOrigin {
        kind: "changed-origin".to_owned(),
        id: "changed-id".to_owned(),
        label: Some("Changed label".to_owned()),
        ..Default::default()
    });
    registry_definition.value_origins.insert(
        "brass_key".to_owned(),
        ProducerOrigin {
            kind: "changed-value".to_owned(),
            id: "changed-value-id".to_owned(),
            label: None,
            ..Default::default()
        },
    );
    registry_definition.producer_fingerprints[0].value = "changed".to_owned();
    assert_eq!(fingerprint, canonical_schema_fingerprint(&registry));

    let mut domain = base.clone();
    let MetadataDomainDefinition::Contextual(domain_definition) = domain
        .metadata_domains
        .get_mut("tone_by_speaker")
        .expect("domain exists")
    else {
        panic!("tone_by_speaker is contextual");
    };
    domain_definition.provenance.origin = Some(ProducerOrigin {
        kind: "changed-origin".to_owned(),
        id: "changed-id".to_owned(),
        label: None,
        ..Default::default()
    });
    domain_definition.provenance.context_origins.clear();
    domain_definition.provenance.value_origins.clear();
    domain_definition.provenance.producer_fingerprints[0].value = "changed".to_owned();
    assert_eq!(fingerprint, canonical_schema_fingerprint(&domain));
}

#[test]
fn schema_freshness_compares_manifest_registry_and_domain_channels_independently() {
    let expected = full_manifest_schema();
    let mut actual = expected.clone();
    assert!(matches!(
        compare_schema_producer_freshness_detailed(&expected, &actual).content_fingerprint,
        ContentFingerprintFreshness::Fresh
    ));

    actual
        .registries
        .get_mut("item")
        .expect("item registry")
        .producer_fingerprints[0]
        .value = "changed-registry".to_owned();
    let MetadataDomainDefinition::Flat(flat) = actual
        .metadata_domains
        .get_mut("tone")
        .expect("tone domain")
    else {
        panic!("tone is flat");
    };
    flat.provenance.producer_fingerprints[0].value = "changed-flat".to_owned();
    let MetadataDomainDefinition::Contextual(contextual) = actual
        .metadata_domains
        .get_mut("tone_by_speaker")
        .expect("tone_by_speaker domain")
    else {
        panic!("tone_by_speaker is contextual");
    };
    contextual.provenance.producer_fingerprints[0].value = "changed-contextual".to_owned();

    let evidence = compare_schema_producer_freshness_detailed(&expected, &actual);
    assert!(matches!(
        evidence.registries["item"],
        ProducerFreshness::Mismatch { .. }
    ));
    assert!(matches!(
        evidence.metadata_domains["tone"],
        ProducerFreshness::Mismatch { .. }
    ));
    assert!(matches!(
        evidence.metadata_domains["tone_by_speaker"],
        ProducerFreshness::Mismatch { .. }
    ));
    assert!(matches!(evidence.manifest, ProducerFreshness::Fresh));
}

#[test]
fn schema_freshness_does_not_collide_manifest_and_scoped_fingerprints() {
    let schema = full_manifest_schema();
    let evidence = compare_schema_producer_freshness_detailed(&schema, &schema);
    assert!(evidence.is_fresh());
}

#[test]
fn legacy_schema_freshness_comparator_reports_scoped_producer_result() {
    let expected = full_manifest_schema();
    let mut actual = expected.clone();
    actual
        .registries
        .get_mut("item")
        .expect("item registry")
        .producer_fingerprints[0]
        .value = "changed-registry".to_owned();

    assert!(matches!(
        compare_schema_producer_freshness(&expected, &actual),
        ProducerFreshness::Mismatch { .. }
    ));
    assert!(matches!(
        compare_schema_producer_freshness_detailed(&expected, &actual).registries["item"],
        ProducerFreshness::Mismatch { .. }
    ));
}

#[test]
fn legacy_schema_freshness_comparator_keeps_content_mismatch_stale() {
    let expected = full_manifest_schema();
    let mut actual = expected.clone();
    actual
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .content_fingerprint = Some(
        recite_core::producer_content_fingerprint(
            "blake3",
            "1111111111111111111111111111111111111111111111111111111111111111",
        )
        .expect("valid content fingerprint"),
    );

    assert!(matches!(
        compare_schema_producer_freshness(&expected, &actual),
        ProducerFreshness::ContentMismatch { .. }
    ));
    assert!(!compare_schema_producer_freshness_detailed(&expected, &actual).is_fresh());
}

#[test]
fn freshness_loader_retains_duplicate_fingerprints_but_ordinary_loader_rejects_them() {
    let source = r#"{
  "schema_version": 1,
  "producer_fingerprints": [
    { "id": "items", "kind": "directory", "algorithm": "blake3", "value": "one" },
    { "id": "items", "kind": "directory", "algorithm": "blake3", "value": "two" }
  ]
}"#;
    let strict = load_schema_manifest_str("strict.json", source);
    assert!(strict.schema.is_none());
    let freshness = recite_core::load_schema_manifest_for_freshness_str("freshness.json", source);
    let schema = freshness
        .schema
        .expect("freshness loader should retain duplicates");
    assert_eq!(
        schema
            .producer_metadata
            .expect("producer metadata")
            .producer_fingerprints
            .len(),
        2
    );
}
