use recite_core::{
    ProducerFreshness, compare_schema_producer_freshness, load_schema_manifest_str,
    load_schema_source_str,
};

#[test]
fn source_fingerprint_ignores_trivia_and_map_order_but_tracks_semantics_and_identity() {
    let first = r#"
schema_version=1
[producer]
id="dialogue"
[types.alpha]
kind="enum"
values=["one", "two"]
[types.beta]
kind="enum"
values=["red"]
"#;
    let reordered = r#"# a comment
schema_version = 1

[producer] # trailing header comment
id = "dialogue"

[types.beta]
kind = "enum"
values = [ "red" ]

[types.alpha]
kind = "enum"
values = ["one", "two"] # value comment
"#;
    let first = load_schema_source_str("first.toml", first)
        .source
        .expect("first source");
    let reordered = load_schema_source_str("reordered.toml", reordered)
        .source
        .expect("reordered source");
    assert_eq!(first.source_fingerprint(), reordered.source_fingerprint());
    assert_eq!(first.schema_fingerprint(), reordered.schema_fingerprint());
    assert_eq!(
        first
            .schema()
            .producer_metadata
            .as_ref()
            .expect("producer metadata")
            .producer_fingerprints
            .len(),
        1
    );

    let changed = load_schema_source_str(
        "changed.toml",
        &reordered.source_text().replace("\"red\"", "\"blue\""),
    )
    .source
    .expect("changed source");
    assert_ne!(first.source_fingerprint(), changed.source_fingerprint());

    let identity = load_schema_source_str(
        "identity.toml",
        &reordered
            .source_text()
            .replace("dialogue", "other-dialogue"),
    )
    .source
    .expect("identity source");
    assert_ne!(first.source_fingerprint(), identity.source_fingerprint());
    assert_eq!(first.schema_fingerprint(), identity.schema_fingerprint());
}

#[test]
fn fixture_export_is_deterministic_and_preserves_stale_linkage() {
    let text = include_str!("../../../../fixtures/schema/valid/standalone.toml");
    let source = load_schema_source_str("standalone.toml", text)
        .source
        .expect("valid standalone source");
    let exported = source.export_json();
    assert_eq!(exported, source.export_json());

    let generated = load_schema_manifest_str("standalone.json", &exported);
    let generated_schema = generated.schema.expect("generated export loads");
    assert!(matches!(
        compare_schema_producer_freshness(source.schema(), &generated_schema),
        ProducerFreshness::Fresh
    ));

    let changed = load_schema_source_str(
        "standalone-changed.toml",
        &text.replace("silver_key", "gold_key"),
    )
    .source
    .expect("changed source remains valid");
    assert!(matches!(
        compare_schema_producer_freshness(source.schema(), changed.schema()),
        ProducerFreshness::Mismatch { .. }
    ));
}

#[test]
fn diagnostic_only_provenance_does_not_change_fingerprints() {
    let text = include_str!("../../../../fixtures/schema/valid/standalone.toml");
    let original = load_schema_source_str("standalone.toml", text)
        .source
        .expect("valid standalone source");
    let changed_provenance = text.replace(
        "[metadata_domains.tone_by_speaker.origin]\nkind = \"data_table\"\nid = \"content/tone.csv\"",
        "[metadata_domains.tone_by_speaker.origin]\nkind = \"data_table\"\nid = \"content/tone-v2.csv\"",
    );
    let changed = load_schema_source_str("standalone-provenance.toml", &changed_provenance)
        .source
        .expect("provenance-only change remains valid");

    assert_eq!(original.source_fingerprint(), changed.source_fingerprint());
    assert_eq!(original.schema_fingerprint(), changed.schema_fingerprint());
    assert!(matches!(
        compare_schema_producer_freshness(original.schema(), changed.schema()),
        ProducerFreshness::Fresh
    ));
    assert_ne!(original.export_json(), changed.export_json());
}
