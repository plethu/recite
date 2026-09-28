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
            "msgctxt \"11111111111111111111\"\n",
            "msgid \"Hello.\"\n",
            "msgstr \"Bonjour.\"\n",
        ),
    )?;
    catalog.import_po(
        "fr",
        "choices.po",
        concat!(
            "msgctxt \"22222222222222222222&formal\"\n",
            "msgid \"Continue.\"\n",
            "msgstr \"Veuillez continuer.\"\n\n",
            "msgctxt \"availability_reason:blocked\"\n",
            "msgid \"Blocked.\"\n",
            "msgstr \"Bloqué.\"\n",
        ),
    )?;
    assert_eq!(
        lookup(
            &catalog,
            "11111111111111111111",
            "Hello.",
            TextDomain::Line,
            None
        )
        .as_deref(),
        Some("Bonjour.")
    );
    assert_eq!(
        lookup(
            &catalog,
            "22222222222222222222",
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
        "msgctxt \"33333333333333333333\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    catalog.import_po(
        "fr",
        "same.po",
        "msgctxt \"33333333333333333333\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n",
    )?;
    let error = catalog
        .import_po(
            "fr",
            "conflict.po",
            concat!(
                "msgctxt \"44444444444444444444\"\nmsgid \"New.\"\nmsgstr \"Nouveau.\"\n\n",
                "msgctxt \"33333333333333333333\"\nmsgid \"Hello.\"\nmsgstr \"Salut.\"\n",
            ),
        )
        .err();
    assert_eq!(
        error.map(|error| error.kind()),
        Some(AdapterErrorKind::Localisation)
    );
    assert_eq!(
        lookup(
            &catalog,
            "33333333333333333333",
            "Hello.",
            TextDomain::Line,
            None
        )
        .as_deref(),
        Some("Bonjour.")
    );
    assert_eq!(
        lookup(
            &catalog,
            "44444444444444444444",
            "New.",
            TextDomain::Line,
            None
        ),
        None
    );
    Ok(())
}
