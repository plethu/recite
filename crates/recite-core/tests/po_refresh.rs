use recite_core::po::{PoDocument, PoEdit, PoRefreshError};

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

#[test]
fn unchanged_refresh_matches_reparse_and_returns_independent_editable_records() {
    let text = concat!(
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=n > 1;\\n\"\n\n",
        "#. block: été\n#: dialogue/été.recite:4:2\n# Translator's note\n#, custom\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"Été\"\nmsgstr \"Summer\"\nextra \"vendor\"\n\n",
        "msgctxt \"22222222222222222222\"\nmsgid \"One\"\nmsgid_plural \"Many\"\nmsgstr[0] \"Un\"\nmsgstr[1] \"Plusieurs\"\n\n",
        "#~ msgctxt \"33333333333333333333\"\n#~ msgid \"Old\"\n#~ msgstr \"Vieux\"\n",
    );
    for ending in ["\n", "\r\n"] {
        let source = text.replace('\n', ending);
        let original = PoDocument::parse_with_path("locale/fr.po", &source).unwrap();
        let mut refreshed = original.refreshed(&original).unwrap();
        let reparsed = PoDocument::parse_with_path("locale/fr.po", &source).unwrap();
        assert_eq!(refreshed, reparsed);
        assert_eq!(refreshed, original);
        let singular = refreshed.find("11111111111111111111", "Été").unwrap().id();
        let plural = refreshed.find("22222222222222222222", "One").unwrap().id();
        refreshed
            .apply_edits([
                PoEdit::translation(singular, "Été traduit"),
                PoEdit::plural_translation(plural, 1, "Beaucoup"),
            ])
            .unwrap();
        assert_eq!(original.source(), source);
        assert_eq!(original, reparsed);
        assert_eq!(
            refreshed.entry(singular).unwrap().translation(),
            Some("Été traduit")
        );
        assert_eq!(
            refreshed.entry(plural).unwrap().plural_translations()[1].text(),
            "Beaucoup"
        );
        assert_eq!(
            refreshed,
            PoDocument::parse_with_path("locale/fr.po", refreshed.source()).unwrap()
        );
    }
}

#[test]
fn refreshing_the_same_template_still_normalizes_extractor_comments() {
    let original = PoDocument::parse(concat!(
        "# Translator's note\n#. block: été\n#: dialogue/été.recite:4:2\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"Été\"\nmsgstr \"Summer\"\n",
    ))
    .unwrap();
    let refreshed = original.refreshed(&original).unwrap();
    assert_eq!(
        refreshed.source(),
        concat!(
            "#. block: été\n#: dialogue/été.recite:4:2\n# Translator's note\n",
            "msgctxt \"11111111111111111111\"\nmsgid \"Été\"\nmsgstr \"Summer\"\n",
        )
    );
    assert_ne!(refreshed, original);
    assert_eq!(refreshed.refreshed(&original).unwrap(), refreshed);
}
