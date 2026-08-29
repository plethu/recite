use recite_core::{
    decode_interpolation_text, extract_placeholder_occurrences, validate_translation_placeholders,
};

#[test]
fn placeholder_validation_preserves_repetition() {
    assert!(validate_translation_placeholders("{name} {name}", "{name}").is_err());
    assert_eq!(
        extract_placeholder_occurrences(r"\{name\} {name}").unwrap(),
        ["name"]
    );
}

#[test]
fn escaped_braces_decode_only_at_runtime_boundary() {
    assert_eq!(
        decode_interpolation_text(r"literal \{brace\}"),
        "literal {brace}"
    );
}
