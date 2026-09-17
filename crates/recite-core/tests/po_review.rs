use recite_core::{PoDocument, PoEntryId};

#[test]
fn review_preserves_unrelated_flags_comments_and_crlf() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# note\r\n#, fuzzy, custom-flag\r\n#. context\r\nmsgctxt \"11111111111111111111\"\r\nmsgid \"Hello\"\r\nmsgstr \"Bonjour\"\r\n";
    let mut document = PoDocument::parse(source)?;
    document.set_fuzzy(PoEntryId::new(0), false)?;
    assert_eq!(document.source(), source.replace("fuzzy, ", ""));
    let reviewed = document.clone();
    document.set_fuzzy(PoEntryId::new(0), true)?;
    assert!(document.source().starts_with("#, fuzzy\r\n"));
    document.set_fuzzy(PoEntryId::new(0), false)?;
    assert_eq!(document, reviewed);
    Ok(())
}
#[test]
fn invalid_translation_cannot_be_marked_reviewed() -> Result<(), Box<dyn std::error::Error>> {
    let source =
        "#, fuzzy\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour\"\n";
    let mut document = PoDocument::parse(source)?;
    assert!(document.set_fuzzy(PoEntryId::new(0), false).is_err());
    assert_eq!(document.source(), source);
    Ok(())
}
