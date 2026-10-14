use recite_writer_model::{Document, FIXTURE};

#[test]
fn owned_passage_mutation_does_not_change_document_or_earlier_snapshots()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(FIXTURE)?;
    let passages = document.passage_snapshot()?;
    let script = document.script_snapshot()?;
    let original = &passages[0];
    let mut copy = original.clone();
    assert_eq!(copy, *original);
    assert_eq!(copy.text.as_ptr(), original.text.as_ptr());
    copy.text = "A separate café draft.".into();
    copy.id = "ffffffffffffffffffff".into();
    assert_eq!(document.find_passage(&original.id)?, Some(original.clone()));
    assert_eq!(document.find_passage(&copy.id)?, None);
    assert_eq!(document.source(), FIXTURE);
    assert_eq!(script, Document::new(FIXTURE)?.script_snapshot()?);
    document.replace_text(document.revision(), &original.id, "New accepted 🐧 prose.")?;
    assert_eq!(passages[0], *original);
    assert_eq!(
        document
            .find_passage(&original.id)?
            .ok_or("missing passage")?
            .text,
        "New accepted 🐧 prose."
    );
    assert_eq!(copy.text, "A separate café draft.");
    assert!(document.undo()?);
    assert_eq!(document.passage_snapshot()?, passages);
    assert_eq!(document.script_snapshot()?, script);
    Ok(())
}
