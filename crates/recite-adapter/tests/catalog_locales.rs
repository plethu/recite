use recite_adapter::{AdapterErrorKind, ReciteDialogueCatalog};
use recite_core::LocaleId;
use recite_runtime::localisation::{LocaleProvider, TextDomain};

fn locale(value: &str) -> Result<LocaleId, Box<dyn std::error::Error>> {
    Ok(LocaleId::new(value)?)
}

#[test]
fn singular_keys_and_queries_share_canonical_locale_and_valid_parent_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.insert("fr-fr", "line", "Hello.", "Bonjour.")?;
    catalog.insert("fr", "parent", "Parent.", "Français.")?;

    assert_eq!(
        catalog.lookup("line", "Hello.", TextDomain::Line, &locale("fr-FR")?, None)?,
        Some("Bonjour.".to_owned())
    );
    assert_eq!(
        catalog.lookup(
            "parent",
            "Parent.",
            TextDomain::Line,
            &locale("fr-FR")?,
            None,
        )?,
        Some("Français.".to_owned())
    );
    let conflict = catalog.insert("fr-FR", "line", "Hello.", "Salut.");
    assert_eq!(
        conflict.err().map(|error| error.kind()),
        Some(AdapterErrorKind::Localisation)
    );
    Ok(())
}

#[test]
fn plural_rule_entries_and_provenance_use_canonical_locale()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.set_plural_forms("fr-fr", "nplurals=2; plural=(n != 1);")?;
    catalog.insert_plural(
        "fr-FR",
        "letters",
        "One letter.",
        "Many letters.",
        vec!["Une lettre.".to_owned(), "Des lettres.".to_owned()],
        None,
    )?;
    let resolved = catalog.resolve_plural(
        "letters",
        "One letter.",
        "Many letters.",
        2,
        TextDomain::Line,
        &locale("fr-fr")?,
        None,
    )?;
    assert_eq!(resolved.template.as_deref(), Some("Des lettres."));
    assert_eq!(resolved.matched_locale.as_deref(), Some("fr-FR"));
    Ok(())
}

#[test]
fn po_import_canonicalizes_locale_and_rejects_malformed_tags()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    let po = "msgctxt \"12345678901234567890\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";
    catalog.import_po("fr-fr", "dialogue.po", po)?;
    assert_eq!(
        catalog.lookup(
            "12345678901234567890",
            "Hello.",
            TextDomain::Line,
            &locale("fr-FR")?,
            None
        )?,
        Some("Bonjour.".to_owned())
    );
    for malformed in ["en--US", "-en", "en-", "not a locale"] {
        assert_eq!(
            catalog
                .import_po(malformed, "dialogue.po", po)
                .err()
                .map(|error| error.kind()),
            Some(AdapterErrorKind::Localisation),
            "{malformed}"
        );
        assert_eq!(
            catalog
                .insert(malformed, "other", "Other.", "Autre.")
                .err()
                .map(|error| error.kind()),
            Some(AdapterErrorKind::Localisation),
            "{malformed}"
        );
    }
    Ok(())
}

#[test]
fn invalid_runtime_locale_never_falls_back_to_valid_parent()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.insert("en", "line", "Hello.", "Translated.")?;
    catalog.set_plural_forms("en", "nplurals=2; plural=(n != 1);")?;
    catalog.insert_plural(
        "en",
        "letters",
        "One letter.",
        "Many letters.",
        vec!["One.".to_owned(), "Many.".to_owned()],
        None,
    )?;
    let malformed = locale("en--US")?;
    assert!(
        catalog
            .lookup("line", "Hello.", TextDomain::Line, &malformed, None)
            .is_err()
    );
    assert!(
        catalog
            .resolve_plural(
                "letters",
                "One letter.",
                "Many letters.",
                2,
                TextDomain::Line,
                &malformed,
                None,
            )
            .is_err()
    );
    Ok(())
}
