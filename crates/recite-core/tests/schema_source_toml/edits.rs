use recite_core::load_schema_source_str;

#[test]
fn typed_edit_preserves_comments_and_updates_source_fingerprint() {
    let source_text = r#"
schema_version = 1

[producer] # ownership metadata
id = "dialogue" # stable identity

[types.mood]
kind = "enum"
values = ["calm"]
"#;
    let mut source = load_schema_source_str("schema.toml", source_text)
        .source
        .expect("valid source");
    let before = source.source_fingerprint().clone();
    source
        .apply_edit(recite_core::SchemaSourceEdit::SetProducerId(
            "dialogue-v2".to_owned(),
        ))
        .expect("producer edit");
    let updated = source.source_text();
    assert!(updated.contains("# ownership metadata"));
    assert!(updated.contains("# stable identity"));
    assert!(updated.contains("dialogue-v2"));
    assert_ne!(&before, source.source_fingerprint());
}

#[test]
fn source_fidelity_preserves_newline_policy_and_enum_decorations() {
    let source_text = "schema_version = 1\r\n\r\n[producer] # owner\r\nid = \"dialogue\" # identity\r\n\r\n[types.mood]\r\nkind = \"enum\" # kind decoration\r\nvalues = [\"calm\"] # array decoration";
    let mut source = load_schema_source_str("fidelity.toml", source_text)
        .source
        .expect("valid source");
    assert_eq!(source.source_text(), source_text);
    source
        .apply_edit(recite_core::SchemaSourceEdit::SetEnumValues {
            name: "mood".to_owned(),
            values: vec!["calm".to_owned(), "tense".to_owned()],
        })
        .expect("enum edit");
    let updated = source.source_text();
    assert!(updated.contains("# owner"));
    assert!(updated.contains("# identity"));
    assert!(updated.contains("# kind decoration"));
    assert!(updated.contains("# array decoration"));
    assert!(updated.contains("values = [\"calm\", \"tense\"]"));
    assert!(!updated.ends_with('\n'));

    let before = updated.clone();
    let error = source.apply_edit(recite_core::SchemaSourceEdit::SetEnumValues {
        name: "mood".to_owned(),
        values: vec![String::new()],
    });
    assert!(error.is_err());
    assert_eq!(source.source_text(), before);
}

#[test]
fn enum_edit_does_not_migrate_removed_element_comments() {
    let source_text = r#"schema_version = 1

[producer]
id = "dialogue"

[types.mood]
kind = "enum"
values = [
  "calm", # retained element
  # removed element comment
  "tense", # removed trailing comment
] # array-level decoration
"#;
    let mut source = load_schema_source_str("enum-comments.toml", source_text)
        .source
        .expect("valid source");
    source
        .apply_edit(recite_core::SchemaSourceEdit::SetEnumValues {
            name: "mood".to_owned(),
            values: vec!["calm".to_owned(), "bright".to_owned()],
        })
        .expect("enum edit");
    let updated = source.source_text();
    assert!(updated.contains("# array-level decoration"));
    assert!(updated.contains("# retained element"));
    assert!(updated.contains("\"calm\", # retained element"));
    assert!(!updated.contains("\"bright\", # retained element"));
    assert!(!updated.contains("removed element comment"));
    assert!(!updated.contains("removed trailing comment"));
    assert!(updated.contains("\"calm\""));
    assert!(updated.contains("\"bright\""));

    let before = updated.clone();
    let error = source.apply_edit(recite_core::SchemaSourceEdit::SetEnumValues {
        name: "mood".to_owned(),
        values: vec!["calm".to_owned(), "calm".to_owned()],
    });
    assert!(error.is_err());
    assert_eq!(source.source_text(), before);
}
