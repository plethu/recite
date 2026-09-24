use recite_core::{PoDocument, PoRefreshError};

#[test]
fn refresh_preserves_translations_notes_unknown_fields_and_marks_source_changes() {
    let original = PoDocument::parse(concat!(
        "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n",
        "# Translator's note\n#. block: old\n#, custom\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\nextra \"vendor\"\n\n",
        "msgctxt \"22222222222222222222\"\nmsgid \"G11111111111111111111\"\nmsgstr \"Parti\"\n"
    )).unwrap();
    let template = PoDocument::parse(concat!(
        "#. block: new\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"\"\n\n",
        "msgctxt \"33333333333333333333\"\nmsgid \"New\"\nmsgstr \"\"\n"
    ))
    .unwrap();
    let refreshed = original.refreshed(&template).unwrap();
    let changed = refreshed
        .entries()
        .iter()
        .find(|e| e.context() == Some("11111111111111111111"))
        .unwrap();
    assert_eq!(changed.translation(), Some("Bonjour"));
    assert_eq!(changed.source_text(), "Hello {name}");
    assert!(changed.flags().iter().any(|f| f == "fuzzy"));
    assert!(refreshed.source().contains("# Translator's note"));
    assert!(refreshed.source().contains("extra \"vendor\""));
    assert!(refreshed.source().contains("#. block: new"));
    assert!(!refreshed.source().contains("#. block: old"));
    assert!(
        refreshed
            .entries()
            .iter()
            .any(|e| e.context() == Some("22222222222222222222")
                && e.is_obsolete()
                && e.translation() == Some("Parti"))
    );
    assert_eq!(
        refreshed
            .find("33333333333333333333", "New")
            .unwrap()
            .translation(),
        Some("")
    );
    assert_eq!(
        refreshed.refreshed(&template).unwrap().source(),
        refreshed.source()
    );
    assert_eq!(
        original
            .find("11111111111111111111", "Hello")
            .unwrap()
            .translation(),
        Some("Bonjour")
    );
}

#[test]
fn variants_follow_their_base_and_do_not_replace_the_default_entry() {
    let original = PoDocument::parse(
        "msgctxt \"11111111111111111111&formal\"\nmsgid \"Hi\"\nmsgstr \"Hello\"\n",
    )
    .unwrap();
    let template =
        PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"Hey\"\nmsgstr \"\"\n")
            .unwrap();
    let result = original.refreshed(&template).unwrap();
    assert_eq!(result.entries().len(), 2);
    assert_eq!(
        result.entries()[0].context(),
        Some("11111111111111111111&formal")
    );
    assert_eq!(result.entries()[0].translation(), Some("Hello"));
    assert_eq!(
        result
            .find("11111111111111111111", "Hey")
            .unwrap()
            .translation(),
        Some("")
    );
}

#[test]
fn ambiguous_contexts_are_refused_without_mutation() {
    let original = PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"Hi\"\nmsgstr \"\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"Hey\"\nmsgstr \"\"\n").unwrap();
    assert!(matches!(
        original.refreshed(&original),
        Err(PoRefreshError::AmbiguousContext(_))
    ));
}

#[test]
fn new_plurals_use_target_arm_count_and_shape_changes_are_explicit() {
    let original = PoDocument::parse(
        "msgid \"\"\nmsgstr \"Plural-Forms: nplurals=3; plural=n == 1 ? 0 : n == 2 ? 1 : 2;\\n\"\n",
    )
    .unwrap();
    let template = PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"One\"\nmsgid_plural \"Many\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n").unwrap();
    let refreshed = original.refreshed(&template).unwrap();
    assert_eq!(refreshed.entries()[1].plural_translations().len(), 3);
    let singular =
        PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"One\"\nmsgstr \"\"\n")
            .unwrap();
    assert!(matches!(
        singular.refreshed(&template),
        Err(PoRefreshError::PluralShape(_))
    ));
    assert!(matches!(
        PoDocument::parse("").unwrap().refreshed(&template),
        Err(PoRefreshError::MissingPluralRule)
    ));
}

#[test]
fn refresh_preserves_crlf_and_existing_obsolete_records() {
    let text = "# note\r\nmsgctxt \"11111111111111111111\"\r\nmsgid \"Hello\"\r\nmsgstr \"Bonjour\"\r\n\r\n#~ msgctxt \"22222222222222222222\"\r\n#~ msgid \"Old\"\r\n#~ msgstr \"Vieux\"\r\n";
    let original = PoDocument::parse(text).unwrap();
    let template =
        PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"\"\n")
            .unwrap();
    assert_eq!(original.refreshed(&template).unwrap().source(), text);
}
