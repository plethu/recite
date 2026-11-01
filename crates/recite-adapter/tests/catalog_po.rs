use recite_adapter::{AdapterErrorKind, ReciteDialogueCatalog};
use recite_core::LocaleId;
use recite_runtime::localisation::{LocaleProvider, TextDomain};

fn lookup(
    catalog: &ReciteDialogueCatalog,
    id: &str,
    source: &str,
    domain: TextDomain,
    variant: Option<&str>,
) -> Option<String> {
    let locale = LocaleId::new("fr-CA").ok()?;
    catalog
        .lookup(id, source, domain, &locale, variant)
        .ok()
        .flatten()
}

#[test]
fn po_import_merges_files_and_resolves_choice_reason_and_variant()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.import_po(
        "fr",
        "lines.po",
        concat!(
            "msgctxt \"ffee6p8\"\n",
            "msgid \"Hello.\"\n",
            "msgstr \"Bonjour.\"\n",
        ),
    )?;
    catalog.import_po(
        "fr",
        "choices.po",
        concat!(
            "msgctxt \"rthh7ab&formal\"\n",
            "msgid \"Continue.\"\n",
            "msgstr \"Veuillez continuer.\"\n\n",
            "msgctxt \"availability_reason:blocked\"\n",
            "msgid \"Blocked.\"\n",
            "msgstr \"Bloqué.\"\n",
        ),
    )?;
    assert_eq!(
        lookup(&catalog, "ffee6p8", "Hello.", TextDomain::Line, None).as_deref(),
        Some("Bonjour.")
    );
    assert_eq!(
        lookup(
            &catalog,
            "rthh7ab",
            "Continue.",
            TextDomain::Choice,
            Some("formal")
        )
        .as_deref(),
        Some("Veuillez continuer.")
    );
    assert_eq!(
        lookup(
            &catalog,
            "blocked",
            "Blocked.",
            TextDomain::AvailabilityReason,
            None
        )
        .as_deref(),
        Some("Bloqué.")
    );
    Ok(())
}

#[test]
fn later_conflict_does_not_partially_merge() -> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.import_po(
        "fr",
        "first.po",
        "msgctxt \"bqyvpqq\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    catalog.import_po(
        "fr",
        "same.po",
        "msgctxt \"bqyvpqq\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    let error = catalog
        .import_po(
            "fr",
            "conflict.po",
            concat!(
                "msgctxt \"z0g6c80\"\nmsgid \"New.\"\nmsgstr \"Nouveau.\"\n\n",
                "msgctxt \"bqyvpqq\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
            ),
        )
        .err();
    assert_eq!(
        error.map(|error| error.kind()),
        Some(AdapterErrorKind::Localisation)
    );
    assert_eq!(
        lookup(&catalog, "bqyvpqq", "Hello.", TextDomain::Line, None).as_deref(),
        Some("Bonjour.")
    );
    assert_eq!(
        lookup(&catalog, "z0g6c80", "New.", TextDomain::Line, None),
        None
    );
    Ok(())
}

#[test]
fn incoming_delta_preserves_first_conflict_and_rolls_back_all_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let mut catalog = ReciteDialogueCatalog::new();
    catalog.insert("fr", "ffee6p8", "Hello.", "Bonjour.")?;
    catalog.insert("fr", "bqyvpqq", "Later.", "Plus tard.")?;
    let before = format!("{catalog:?}");
    let error = catalog
        .import_po(
            "fr",
            "conflict.po",
            concat!(
                "msgctxt \"rthh7ab\"\nmsgid \"Goodbye.\"\nmsgstr \"Au revoir.\"\n\n",
                "msgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Different.\"\n\n",
                "msgctxt \"bqyvpqq\"\nmsgid \"Later.\"\nmsgstr \"Also different.\"\n"
            ),
        )
        .expect_err("first conflict rejects the entire document");
    assert!(
        error.to_string().contains("conflicting catalogue entry"),
        "{error}"
    );
    assert!(error.to_string().contains("ffee6p8"), "{error}");
    assert_eq!(format!("{catalog:?}"), before);
    let error = catalog.import_po(
        "fr",
        "internal.po",
        concat!(
            "msgctxt \"rthh7ab\"\nmsgid \"Goodbye.\"\nmsgstr \"One.\"\n\n",
            "msgctxt \"rthh7ab\"\nmsgid \"Goodbye.\"\nmsgstr \"Two.\"\n"
        ),
    );
    assert!(error.is_err());
    assert_eq!(format!("{catalog:?}"), before);
    Ok(())
}
