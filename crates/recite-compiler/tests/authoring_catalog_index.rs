use recite_compiler::authoring::CatalogInput;
use recite_core::{LocaleId, po::PoDocument};

#[test]
fn indexed_matches_preserve_all_lossless_records_and_key_boundaries()
-> Result<(), Box<dyn std::error::Error>> {
    let po = concat!(
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\n",
        "#, fuzzy\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Brouillon\"\n\n",
        "#~ msgctxt \"11111111111111111111\"\n#~ msgid \"Hello\"\n#~ msgstr \"Ancien\"\n\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n\n",
        "msgctxt \"11111111111111111111&formal\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\n",
        "msgctxt \"22222222222222222222\"\nmsgid \"Hello\"\nmsgid_plural \"Greetings\"\nmsgstr[0] \"Bonjour\"\nmsgstr[1] \"Salutations\"\n",
    );
    let input = CatalogInput::from_document(LocaleId::new("fr")?, PoDocument::parse(po)?)?;
    let retained = input.clone();
    assert!(std::ptr::eq(input.document(), retained.document()));
    for entry in input.document().entries() {
        for context in [entry.context().unwrap_or_default(), "missing"] {
            for source in [entry.source_text(), "mismatch"] {
                for plural in [entry.plural_source_text(), Some("mismatched plural"), None] {
                    let expected = input
                        .document()
                        .entries()
                        .iter()
                        .filter(|entry| {
                            !entry.is_header()
                                && entry.context() == Some(context)
                                && entry.source_text() == source
                                && entry.plural_source_text() == plural
                        })
                        .map(|entry| entry.id())
                        .collect::<Vec<_>>();
                    let actual = input
                        .matching_entries(context, source, plural)
                        .map(|entry| entry.id())
                        .collect::<Vec<_>>();
                    assert_eq!(actual, expected);
                }
            }
        }
    }
    let mut edited = input.document().clone();
    edited.apply_edit(recite_core::po::PoEdit::translation(
        input.document().entries()[3].id(),
        "Bonjour",
    ))?;
    assert_ne!(edited, *retained.document());
    assert_eq!(
        retained
            .matching_entries("11111111111111111111", "Hello", None)
            .count(),
        3
    );
    Ok(())
}
