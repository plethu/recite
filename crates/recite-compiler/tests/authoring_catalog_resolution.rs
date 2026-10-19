#[path = "support/catalog_resolution.rs"]
mod catalog_resolution;
use catalog_resolution::build;
use recite_compiler::authoring::{CatalogInput, CatalogResolutionPolicy, CatalogSummaryError};
use recite_core::{
    LocaleId,
    po::{PoDocument, PotDocument, PotEntry},
};

#[test]
fn resolution_only_keeps_expected_catalogue_and_policy_error_precedence()
-> Result<(), Box<dyn std::error::Error>> {
    let entry = PotEntry {
        context: "11111111111111111111".into(),
        source_text: "Hello".into(),
        plural_source_text: None,
        comments: Vec::new(),
        reference: None,
    };
    let good = PotDocument {
        entries: vec![entry.clone()],
    };
    let locale = LocaleId::new("fr")?;
    let catalog = CatalogInput::from_document(
        locale.clone(),
        PoDocument::parse("msgid \"\"\nmsgstr \"Language: fr\\n\"\n")?,
    )?;
    let policy = CatalogResolutionPolicy::new(Some(locale.clone())).with_fallback_locale(locale);
    let mut no_context = entry.clone();
    no_context.context.clear();
    let mut no_source = entry.clone();
    no_source.source_text.clear();
    let mut no_plural = entry;
    no_plural.plural_source_text = Some(String::new());
    for (entry, wanted) in [
        (no_context, CatalogSummaryError::EmptyExpectedContext),
        (no_source, CatalogSummaryError::EmptyExpectedSourceText),
        (
            no_plural,
            CatalogSummaryError::EmptyExpectedPluralSourceText,
        ),
    ] {
        assert_eq!(
            build(
                &PotDocument {
                    entries: vec![entry]
                },
                [catalog.clone(), catalog.clone()],
                policy.clone()
            )
            .err(),
            Some(wanted)
        );
    }
    assert!(matches!(
        build(&good, [catalog.clone(), catalog], policy.clone()),
        Err(CatalogSummaryError::DuplicateCatalog { .. })
    ));
    assert!(matches!(
        build(&good, [], policy),
        Err(CatalogSummaryError::FallbackCycle { .. })
    ));
    Ok(())
}
