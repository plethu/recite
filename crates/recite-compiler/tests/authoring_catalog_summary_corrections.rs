#![cfg(test)]

use recite_compiler::{
    CatalogCoverageSummary, CatalogIdentity, CatalogInput, PotDocument, PotEntry, TranslationStatus,
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

fn translated_po(language: &str) -> String {
    format!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: {language}\\n\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n"
    )
}

#[test]
fn locale_truncation_precedes_default_and_named_variant_is_major() {
    let variant = concat!(
        "msgid \"\"\nmsgstr \"\"\n\"Language: pt\\n\"\n\n",
        "msgctxt \"11111111111111111111&formal\"\n",
        "msgid \"Hello\"\nmsgstr \"Olá, formal\"\n",
    );
    let policy = recite_compiler::CatalogResolutionPolicy::new(Some(locale("PT_br")))
        .with_default_locale(locale("de"))
        .with_variant("formal")
        .expect("variant policy");
    assert_eq!(
        policy
            .variants()
            .iter()
            .map(|variant| variant.name())
            .collect::<Vec<_>>(),
        [Some("formal"), None]
    );
    let summary = CatalogCoverageSummary::build(&expected(), [input("pt", "pt", variant)], policy)
        .expect("summary");
    let candidates = summary.resolution().candidates();
    assert_eq!(
        candidates
            .iter()
            .map(|candidate| (candidate.locale().as_str(), candidate.variant().name()))
            .collect::<Vec<_>>(),
        [
            ("pt-BR", Some("formal")),
            ("pt", Some("formal")),
            ("de", Some("formal")),
            ("pt-BR", None),
            ("pt", None),
            ("de", None),
        ]
    );
    let matched = summary.entries()[0].matched().expect("variant match");
    assert_eq!(matched.catalog().locale().as_str(), "pt");
    assert_eq!(matched.candidate().variant().name(), Some("formal"));
}

#[test]
fn locale_identity_is_canonical_and_po_language_must_match() {
    let identity = CatalogIdentity::new("fr", locale("FR_fr")).expect("canonical identity");
    assert_eq!(identity.locale().as_str(), "fr-FR");
    let canonical = input("fr", "FR_fr", &translated_po("FR-fr"));
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [canonical],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr-fr"))),
    )
    .expect("canonical summary");
    assert_eq!(summary.catalogs()[0].locale().as_str(), "fr-FR");

    let mislabeled = CatalogCoverageSummary::build(
        &expected(),
        [input("de", "de", &translated_po("fr"))],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("de"))),
    );
    assert!(matches!(
        mislabeled,
        Err(recite_compiler::CatalogSummaryError::CatalogLocaleMismatch { .. })
    ));
    assert!(matches!(
        CatalogIdentity::new("invalid", locale("not a locale")),
        Err(recite_compiler::CatalogSummaryError::InvalidLocale { .. })
    ));
}

#[test]
fn stale_fuzzy_and_obsolete_plural_records_remain_in_lossless_inventory() {
    let source = concat!(
        "msgid \"\"\nmsgstr \"\"\n",
        "\"Language: fr\\n\"\n",
        "\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n\n",
        "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n\n",
        "#, fuzzy\n",
        "msgctxt \"99999999999999999999\"\n",
        "msgid \"Old one\"\nmsgid_plural \"Old many\"\n",
        "msgstr[0] \"Ancien\"\nmsgstr[1] \"\"\n\n",
        "#~ msgctxt \"88888888888888888888\"\n",
        "#~ msgid \"Removed one\"\n#~ msgid_plural \"Removed many\"\n",
        "#~ msgstr[0] \"Retire\"\n#~ msgstr[1] \"Retires\"\n",
    );
    let summary = CatalogCoverageSummary::build(
        &expected(),
        [input("fr", "fr", source)],
        recite_compiler::CatalogResolutionPolicy::new(Some(locale("fr"))),
    )
    .expect("summary");
    let catalog = &summary.catalogs()[0];
    assert_eq!(catalog.records().len(), 3);
    assert_eq!(catalog.coverage().fuzzy_count(), 1);
    assert_eq!(catalog.coverage().obsolete_count(), 1);
    assert_eq!(catalog.coverage().incomplete_plural_count(), 1);
    let stale = catalog
        .records()
        .iter()
        .find(|record| record.context() == Some("99999999999999999999"))
        .expect("stale fuzzy record");
    assert!(stale.is_fuzzy());
    assert!(matches!(
        stale.translation(),
        TranslationStatus::IncompletePlural {
            expected_arms: Some(2),
            present_arms: 2,
            translated_arms: 1,
        }
    ));
    assert!(catalog.records().iter().any(|record| {
        record.context() == Some("88888888888888888888") && record.is_obsolete()
    }));
}
