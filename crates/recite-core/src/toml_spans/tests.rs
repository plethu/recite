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

#[test]
fn path_indices_preserve_first_source_order_ranges_and_occurrence_search() {
    let source = "# 😀 before\n\"a.b\" = { x = 1.0, y = [2.0, { z = 3.0 }] }\r\n\
        [[scenes]]\npath = \"é.recite\"\n[[scenes]]\npath = \"é.recite\"\n";
    let document = toml_edit::Document::parse(source.to_owned()).unwrap();
    let index = TomlSpanIndex::from_document(&document);
    for entry in &index.entries {
        let path = &entry.path;
        let first = index
            .entries
            .iter()
            .find(|entry| &entry.path == path)
            .unwrap();
        assert_eq!(index.key_range(path), first.key_range);
        assert_eq!(index.value_range(path), first.value_range);
        let float = index
            .entries
            .iter()
            .find(|e| &e.path == path && e.is_float)
            .and_then(|e| e.value_range.clone());
        assert_eq!(index.float_range(path), float);
    }
    for table in &index.tables {
        assert_eq!(
            index.table_range(&table.path),
            index
                .tables
                .iter()
                .find(|t| t.path == table.path)
                .and_then(|t| t.range.clone())
        );
    }
    let first = index.find(Some("scenes"), "é.recite", true, 0).unwrap();
    let second = index
        .find(Some("scenes"), "é.recite", true, first.end)
        .unwrap();
    assert!(first.end < second.start);
    assert!(index.key_range(&["absent".into()]).is_none());
    for offset in 0..source.len() + 2 {
        assert_eq!(
            index.position(offset),
            crate::source_location::position_for_byte_offset(source, offset)
        );
    }
}
