use super::*;
#[test]
fn changes_use_supplied_notes_and_same_beat_source_context() {
    let old=PoDocument::parse("# Translator says: keep this short\nmsgctxt \"11111111111111111111\"\nmsgid \"Nine\"\nmsgstr \"Neuf\"\n").expect("old");
    let template=PoDocument::parse(concat!(
        "#. file: scene.recite\n#. block: tram\n#. speaker: Mara\nmsgctxt \"11111111111111111111\"\nmsgid \"Eight\"\nmsgstr \"\"\n\n",
        "#. file: scene.recite\n#. block: tram\nmsgctxt \"22222222222222222222\"\nmsgid \"Will you come?\"\nmsgstr \"\"\n\n",
        "#. file: other.recite\n#. block: tram\nmsgctxt \"33333333333333333333\"\nmsgid \"Elsewhere\"\nmsgstr \"\"\n"
    )).expect("template");
    let next = old.refreshed(&template).expect("refresh");
    let changes = collect(&old, &next, &template);
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].caption, "scene.recite · tram · Mara");
    assert_eq!(changes[0].nearby, vec!["Will you come?"]);
    assert_eq!(changes[0].notes, vec!["Translator says: keep this short"]);
    assert!(changes[2].nearby.is_empty());
    assert_eq!(changes[0].translation, "Neuf");
}
