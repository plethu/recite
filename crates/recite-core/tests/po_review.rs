use recite_core::po::{PoDocument, PoEntryId};

#[test]
fn review_preserves_unrelated_flags_comments_and_crlf() -> Result<(), Box<dyn std::error::Error>> {
    let source = "# note\r\n#, fuzzy, custom-flag\r\n#. context\r\nmsgctxt \"ffee6p8\"\r\nmsgid \"Hello\"\r\nmsgstr \"Bonjour\"\r\n";
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
    let source = "#, fuzzy\nmsgctxt \"ffee6p8\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour\"\n";
    let mut document = PoDocument::parse(source)?;
    assert!(document.set_fuzzy(PoEntryId::new(0), false).is_err());
    assert_eq!(document.source(), source);
    Ok(())
}

#[test]
fn review_state_follows_translation_fuzzy_flag_and_previous_source()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_core::po::PoReviewState::{NeedsReview, Reviewed, SourceChanged, Untranslated};
    let source = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\n",
        "msgctxt \"blank\"\nmsgid \"Hello\"\nmsgstr \"\"\n\n",
        "msgctxt \"done\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\n",
        "#, fuzzy\nmsgctxt \"draft\"\nmsgid \"Hello\"\nmsgstr \"Salut\"\n\n",
        "#, fuzzy\n#| msgid \"Hi\"\nmsgctxt \"changed\"\nmsgid \"Hello\"\nmsgstr \"Salut\"\n\n",
        "msgctxt \"plural\"\nmsgid \"one\"\nmsgid_plural \"many\"\nmsgstr[0] \"un\"\nmsgstr[1] \"\"\n",
    );
    let document = PoDocument::parse(source)?;
    let states: Vec<_> = document
        .entries()
        .iter()
        .filter(|entry| entry.context().is_some())
        .map(|entry| entry.review_state())
        .collect();
    assert_eq!(
        states,
        [
            Untranslated,
            Reviewed,
            NeedsReview,
            SourceChanged,
            Untranslated
        ]
    );
    assert!(!Reviewed.needs_attention());
    assert!(SourceChanged.needs_attention());
    Ok(())
}
