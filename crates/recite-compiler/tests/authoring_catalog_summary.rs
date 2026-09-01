#![cfg(test)]

use recite_compiler::{
    CatalogCoverageSummary, CatalogIdentity, CatalogInput, CatalogVariant, PotDocument, PotEntry,
    TranslationStatus,
};
use recite_core::{LocaleId, PoDocument};

fn locale(value: &str) -> LocaleId {
    LocaleId::new(value).expect("test locale")
}

fn expected() -> PotDocument {
    PotDocument {
        entries: vec![
            PotEntry {
                context: "11111111111111111111".to_owned(),
                source_text: "Hello".to_owned(),
                plural_source_text: None,
                comments: Vec::new(),
                reference: None,
            },
            PotEntry {
                context: "22222222222222222222".to_owned(),
                source_text: "One letter".to_owned(),
                plural_source_text: Some("Many letters".to_owned()),
                comments: Vec::new(),
                reference: None,
            },
        ],
    }
}

fn input(id: &str, locale: &str, source: &str) -> CatalogInput {
    CatalogInput::new(
        CatalogIdentity::new(id, self::locale(locale)).expect("test identity"),
        PoDocument::parse_with_path(id, source).expect("test PO"),
    )
}

fn translated_po(language: &str, hello: &str, plural_arms: &[&str]) -> String {
    let arms = plural_arms
        .iter()
        .enumerate()
        .map(|(index, value)| format!("msgstr[{index}] \"{value}\"\n"))
        .collect::<String>();
    format!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: {language}\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"{hello}\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"One letter\"\nmsgid_plural \"Many letters\"\n{arms}"
    )
}

#[test]
fn complete_catalogue_exposes_identity_fingerprint_and_counts() {
    let catalog = input(
        "fr",
        "fr-FR",
        &translated_po("fr-FR", "Bonjour", &["Une lettre", "Plusieurs lettres"]),
    );
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [catalog.clone()],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr-FR"))),
    )
    .expect("summary");

    assert_eq!(summary.expected_count(), 2);
    assert_eq!(summary.catalogs().len(), 1);
    assert_eq!(summary.catalogs()[0].id(), "fr");
    assert_eq!(summary.catalogs()[0].locale().as_str(), "fr-FR");
    assert_eq!(
        summary.catalogs()[0].fingerprint(),
        &catalog.document().fingerprint()
    );
    assert_eq!(summary.catalogs()[0].plural_forms(), Some(2));
    assert_eq!(summary.catalogs()[0].coverage().present_count(), 2);
    assert_eq!(summary.catalogs()[0].coverage().translated_count(), 2);
    assert_eq!(summary.catalogs()[0].coverage().missing_count(), 0);
    assert_eq!(summary.resolution().candidates().len(), 2);
    assert_eq!(
        summary.entries()[0]
            .matched()
            .expect("match")
            .catalog()
            .id(),
        "fr"
    );
    assert!(!summary.entries()[0].source_fallback());
}

#[test]
fn missing_fuzzy_obsolete_and_incomplete_plural_remain_visible() {
    let source = concat!(
        "msgid \"\"\nmsgstr \"\"\n",
        "\"Language: fr\\n\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "#, fuzzy\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\n",
        "msgctxt \"22222222222222222222\"\n",
        "msgid \"One letter\"\n",
        "msgid_plural \"Many letters\"\n",
        "msgstr[0] \"Une lettre\"\n",
        "msgstr[1] \"\"\n\n",
        "#~ msgctxt \"22222222222222222222\"\n",
        "#~ msgid \"One letter\"\n",
        "#~ msgid_plural \"Many letters\"\n",
        "#~ msgstr[0] \"Une lettre\"\n",
        "#~ msgstr[1] \"Plusieurs lettres\"\n",
    );
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [input("fr", "fr", source)],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))),
    )
    .expect("summary");
    let coverage = summary.catalogs()[0].coverage();
    assert_eq!(coverage.present_count(), 2);
    assert_eq!(coverage.translated_count(), 0);
    assert_eq!(coverage.missing_count(), 2);
    assert_eq!(coverage.fuzzy_count(), 1);
    assert_eq!(coverage.obsolete_count(), 1);
    assert!(matches!(
        summary.catalogs()[0].entries()[0].translation(),
        TranslationStatus::Untranslated
    ));
    assert!(summary.catalogs()[0].entries()[0].is_fuzzy());
    assert!(summary.catalogs()[0].entries()[0].is_missing());
    assert!(summary.catalogs()[0].entries()[1].is_obsolete());
    assert!(matches!(
        summary.catalogs()[0].entries()[1].translation(),
        TranslationStatus::IncompletePlural {
            expected_arms: Some(2),
            present_arms: 2,
            translated_arms: 1,
        }
    ));
    assert!(summary.catalogs()[0].entries()[1].is_missing());
    assert_eq!(coverage.incomplete_plural_count(), 1);
}

#[test]
fn variants_contexts_and_explicit_fallback_are_deterministic() {
    let variant = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\n",
        "msgctxt \"11111111111111111111&formal\"\n",
        "msgid \"Hello\"\n",
        "msgstr \"Bonjour, formel\"\n",
    );
    let fallback = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: de\\n\"\n\n",
        "msgctxt \"11111111111111111111\"\n",
        "msgid \"Hello\"\n",
        "msgstr \"Hallo\"\n",
    );
    let policy = recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr-CA")))
        .with_default_locale(locale("fr"))
        .with_fallback_locale(locale("de"))
        .with_variants([
            CatalogVariant::named("formal").expect("variant"),
            CatalogVariant::Base,
        ])
        .expect("policy");
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [input("de", "de", fallback), input("fr", "fr", variant)],
        policy,
    )
    .expect("summary");
    assert_eq!(
        summary
            .catalogs()
            .iter()
            .map(|catalog| catalog.id())
            .collect::<Vec<_>>(),
        ["de", "fr"]
    );
    assert_eq!(
        summary.resolution().candidates()[0].locale().as_str(),
        "fr-CA"
    );
    assert!(matches!(
        summary.resolution().candidates()[0].variant(),
        CatalogVariant::Named(name) if name == "formal"
    ));
    let matched = summary.entries()[0].matched().expect("variant match");
    assert_eq!(matched.catalog().id(), "fr");
    assert_eq!(matched.candidate().locale().as_str(), "fr");
    assert!(matches!(
        matched.candidate().variant(),
        CatalogVariant::Named(name) if name == "formal"
    ));

    let reordered = CatalogCoverageSummary::build(
        &expected(),
        [input("fr", "fr", variant), input("de", "de", fallback)],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr-CA")))
            .with_default_locale(locale("fr"))
            .with_fallback_locale(locale("de"))
            .with_variants([
                CatalogVariant::named("formal").expect("variant"),
                CatalogVariant::Base,
            ])
            .expect("policy"),
    )
    .expect("reordered summary");
    assert_eq!(summary, reordered);
}

#[test]
fn source_only_policy_has_no_candidates_and_uses_source_fallback() {
    let summary = CatalogCoverageSummary::build(
        &expected(),
        std::iter::empty(),
        recite_compiler::CatalogResolutionPolicy::source_only(),
    )
    .expect("source-only summary");
    assert!(summary.resolution().is_source_only());
    assert!(summary.resolution().candidates().is_empty());
    assert!(
        summary
            .entries()
            .iter()
            .all(|entry| entry.source_fallback())
    );
}

#[test]
fn split_headerless_catalogues_share_an_explicit_locale() {
    let first = input(
        "a-fr",
        "fr",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n",
    );
    let second = input(
        "z-fr",
        "fr",
        "msgid \"\"\nmsgstr \"\"\n\"Language: fr\\n\"\n\nmsgctxt \"99999999999999999999\"\nmsgid \"Stale\"\nmsgstr \"Ancien\"\n",
    );
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [first.clone(), second.clone()],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))),
    )
    .expect("split headerless catalogues are valid");
    assert_eq!(summary.catalogs().len(), 2);
    assert_eq!(summary.catalogs()[0].id(), "a-fr");
    assert_eq!(summary.catalogs()[1].id(), "z-fr");
    assert_eq!(
        summary.catalogs()[0].locale(),
        summary.catalogs()[1].locale()
    );
    assert_eq!(summary.catalogs()[0].plural_forms(), None);
    assert_eq!(
        summary.entries()[0]
            .matched()
            .expect("split match")
            .catalog()
            .id(),
        "a-fr"
    );
    assert!(summary.entries()[1].source_fallback());

    let reordered = CatalogCoverageSummary::build(
        &expected(),
        [second, first],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))),
    )
    .expect("reordered split catalogues are valid");
    assert_eq!(summary, reordered);
}

#[test]
fn conflicting_catalogue_records_and_candidates_are_rejected() {
    let conflicting_catalogues = CatalogCoverageSummary::build(
        &expected(),
        [
            input(
                "a-fr",
                "fr",
                "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"A\"\n",
            ),
            input(
                "z-fr",
                "fr",
                "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"B\"\n",
            ),
        ],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))),
    );
    assert!(matches!(
        conflicting_catalogues,
        Err(recite_compiler::CatalogSummaryError::CatalogEntryConflict {
            context,
            source_text,
            ..
        }) if context == "11111111111111111111" && source_text == "Hello"
    ));

    let duplicate_variants = recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr")));
    assert!(matches!(
        duplicate_variants.with_variants([CatalogVariant::Base, CatalogVariant::Base]),
        Err(recite_compiler::CatalogSummaryError::DuplicateCandidate { .. })
    ));
    assert!(matches!(
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))).with_variants([]),
        Err(recite_compiler::CatalogSummaryError::EmptyVariantCandidates)
    ));

    let fallback_cycle = CatalogCoverageSummary::build(
        &expected(),
        std::iter::empty(),
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr")))
            .with_fallback_locale(locale("fr")),
    );
    assert!(matches!(
        fallback_cycle,
        Err(recite_compiler::CatalogSummaryError::FallbackCycle { .. })
    ));

    assert!(matches!(
        CatalogIdentity::new(" ", locale("fr")),
        Err(recite_compiler::CatalogSummaryError::EmptyCatalogIdentity)
    ));
}
