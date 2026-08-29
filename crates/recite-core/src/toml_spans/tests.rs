use super::TomlSpanIndex;

fn token<'a>(source: &'a str, index: &TomlSpanIndex, path: &[&str]) -> &'a str {
    let path = path
        .iter()
        .map(|part| (*part).to_owned())
        .collect::<Vec<_>>();
    let range = index
        .float_range(&path)
        .unwrap_or_else(|| panic!("missing float span for {path:?}"));
    source
        .get(range)
        .unwrap_or_else(|| panic!("invalid float span for {path:?}"))
}

#[test]
fn nested_arrays_inline_tables_comments_and_dotted_keys_keep_float_spans_aligned() {
    let source = r#"# before the inline table
root = { nested = [
  [1_234.5_0, -0.0], # first nested array
  [{ deep = +6.0 }],
] } # after the value

[outer]
inner.float = 7.0 # dotted key
"#;
    let document = toml_edit::Document::parse(source.to_owned())
        .unwrap_or_else(|error| panic!("test TOML should parse: {}", error.message()));
    let index = TomlSpanIndex::from_document(&document);

    assert_eq!(
        token(source, &index, &["root", "nested", "[0]", "[0]"]),
        "1_234.5_0"
    );
    assert_eq!(
        token(source, &index, &["root", "nested", "[0]", "[1]"]),
        "-0.0"
    );
    assert_eq!(
        token(source, &index, &["root", "nested", "[1]", "[0]", "deep"]),
        "+6.0"
    );
    assert_eq!(token(source, &index, &["outer", "inner", "float"]), "7.0");
}
