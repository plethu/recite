#[test]
fn stable_ids_are_dimmed_in_writer_only() {
    assert!(recite_writer_grammar::HIGHLIGHTS.contains(super::STABLE_ID_CAPTURE));
    assert!(super::HIGHLIGHTS.contains("((stable_id_field) @comment)"));
    assert!(!super::HIGHLIGHTS.contains(super::STABLE_ID_CAPTURE));
}
