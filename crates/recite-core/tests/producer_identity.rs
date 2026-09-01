use recite_core::ProducerIdentity;

#[test]
fn producer_identity_has_one_validated_constructor_and_wire_shape() {
    let identity = ProducerIdentity::new("adapter", "example").expect("valid identity");
    assert_eq!(identity.kind(), "adapter");
    assert_eq!(identity.id(), "example");

    let encoded = serde_json::to_string(&identity).expect("serialize identity");
    assert_eq!(encoded, r#"{"kind":"adapter","id":"example"}"#);
    let decoded: ProducerIdentity = serde_json::from_str(&encoded).expect("deserialize identity");
    assert_eq!(decoded, identity);
}

#[test]
fn producer_identity_rejects_empty_and_whitespace_components() {
    for (kind, id) in [("", "id"), ("   ", "id"), ("kind", ""), ("kind", " \t ")] {
        assert!(
            ProducerIdentity::new(kind, id).is_err(),
            "invalid identity should fail: {kind:?}/{id:?}"
        );
        let wire = format!(r#"{{"kind":{kind:?},"id":{id:?}}}"#);
        assert!(
            serde_json::from_str::<ProducerIdentity>(&wire).is_err(),
            "invalid wire identity should fail: {wire}"
        );
    }
}

#[test]
fn producer_identity_rejects_unknown_wire_fields() {
    let wire = r#"{"kind":"adapter","id":"example","label":"extra"}"#;
    assert!(serde_json::from_str::<ProducerIdentity>(wire).is_err());
}

#[test]
fn manifest_lowering_rejects_whitespace_identity_components() {
    let report = recite_core::load_schema_manifest_str(
        "whitespace-producer.json",
        r#"{"schema_version":1,"producer":{"kind":" \t ","id":"valid"}}"#,
    );
    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics[0].code.as_str(), "RECITE_SCHEMA001");
}
