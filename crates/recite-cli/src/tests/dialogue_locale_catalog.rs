use std::path::PathBuf;

use recite_core::LocaleId;
use recite_runtime::{LocaleProvider, PluralResolutionOutcome, TextDomain};
use tempfile::TempDir;

use crate::dialogue_locale::{DialogueCatalogProvider, DialogueCatalogSource};
use crate::error::CliError;

pub(super) fn lookup_exhausts_variant_locale_fallback_before_base_context() {
    let temp = TempDir::new().expect("tempdir");
    let region = write_catalog(
        temp.path().join("fr-CA.po"),
        catalog(
            header("fr-CA", None),
            [
                singular_entry(None, "11111111111111111111&formal", "Line source", ""),
                singular_entry(
                    Some("fuzzy"),
                    "22222222222222222222&formal",
                    "Choice source",
                    "Fuzzy Canadian choice",
                ),
                singular_entry(None, "11111111111111111111", "Line source", "Canadian line"),
                singular_entry(
                    None,
                    "22222222222222222222",
                    "Choice source",
                    "Canadian choice",
                ),
                singular_entry(
                    None,
                    "availability_reason:33333333333333333333",
                    "Availability source",
                    "Canadian availability",
                ),
            ],
        ),
    );
    let language = write_catalog(
        temp.path().join("fr.po"),
        catalog(
            header("fr", None),
            [
                singular_entry(
                    None,
                    "11111111111111111111&formal",
                    "Line source",
                    "French formal line",
                ),
                singular_entry(
                    None,
                    "22222222222222222222&formal",
                    "Choice source",
                    "French formal choice",
                ),
                singular_entry(
                    None,
                    "availability_reason:33333333333333333333&formal",
                    "Availability source",
                    "French formal availability",
                ),
            ],
        ),
    );
    let provider = DialogueCatalogProvider::load(vec![
        source(LocaleId::new("fr-CA").expect("locale"), region),
        source(LocaleId::new("fr").expect("locale"), language),
    ])
    .expect("catalogues load");
    let locale = LocaleId::new("fr-CA").expect("locale");

    assert_eq!(
        provider
            .lookup(
                "11111111111111111111",
                "Line source",
                TextDomain::Line,
                &locale,
                Some("formal"),
            )
            .expect("line lookup")
            .as_deref(),
        Some("French formal line")
    );
    assert_eq!(
        provider
            .lookup(
                "22222222222222222222",
                "Choice source",
                TextDomain::Choice,
                &locale,
                Some("formal"),
            )
            .expect("choice lookup")
            .as_deref(),
        Some("French formal choice")
    );
    assert_eq!(
        provider
            .lookup(
                "33333333333333333333",
                "Availability source",
                TextDomain::AvailabilityReason,
                &locale,
                Some("formal"),
            )
            .expect("availability lookup")
            .as_deref(),
        Some("French formal availability")
    );
}

pub(super) fn plural_resolution_exhausts_variant_locale_fallback_before_base_context() {
    let temp = TempDir::new().expect("tempdir");
    let region = write_catalog(
        temp.path().join("fr-CA.po"),
        catalog(
            header("fr-CA", Some("nplurals=2; plural=(n != 1);")),
            [
                plural_entry(
                    None,
                    "aaaaaaaaaaaaaaaaaaaa&formal",
                    "one",
                    "many",
                    &["", ""],
                ),
                plural_entry(
                    None,
                    "aaaaaaaaaaaaaaaaaaaa",
                    "one",
                    "many",
                    &["Canadian a one", "Canadian a many"],
                ),
                plural_entry(
                    Some("fuzzy"),
                    "bbbbbbbbbbbbbbbbbbbb&formal",
                    "one",
                    "many",
                    &["Fuzzy Canadian b one", "Fuzzy Canadian b many"],
                ),
                plural_entry(
                    None,
                    "bbbbbbbbbbbbbbbbbbbb",
                    "one",
                    "many",
                    &["Canadian b one", "Canadian b many"],
                ),
                plural_entry(
                    None,
                    "cccccccccccccccccccc",
                    "one",
                    "many",
                    &["Canadian c one", "Canadian c many"],
                ),
            ],
        ),
    );
    let language = write_catalog(
        temp.path().join("fr.po"),
        catalog(
            header(
                "fr",
                Some("nplurals=3; plural=(n == 0 ? 0 : n == 1 ? 1 : 2);"),
            ),
            [
                plural_entry(
                    None,
                    "aaaaaaaaaaaaaaaaaaaa&formal",
                    "one",
                    "many",
                    &["French a zero", "French a one", "French a many"],
                ),
                plural_entry(
                    None,
                    "bbbbbbbbbbbbbbbbbbbb&formal",
                    "one",
                    "many",
                    &["French b zero", "French b one", "French b many"],
                ),
                plural_entry(
                    None,
                    "cccccccccccccccccccc&formal",
                    "one",
                    "many",
                    &["French c zero", "French c one", "French c many"],
                ),
            ],
        ),
    );
    let provider = DialogueCatalogProvider::load(vec![
        source(LocaleId::new("fr-CA").expect("locale"), region),
        source(LocaleId::new("fr").expect("locale"), language),
    ])
    .expect("catalogues load");
    let locale = LocaleId::new("fr-CA").expect("locale");

    assert_plural_variant(
        &provider,
        &locale,
        "aaaaaaaaaaaaaaaaaaaa",
        "French a many",
        PluralResolutionOutcome::MissingTranslation,
    );
    assert_plural_variant(
        &provider,
        &locale,
        "bbbbbbbbbbbbbbbbbbbb",
        "French b many",
        PluralResolutionOutcome::MissingEntry,
    );
    assert_plural_variant(
        &provider,
        &locale,
        "cccccccccccccccccccc",
        "French c many",
        PluralResolutionOutcome::MissingEntry,
    );
}

pub(super) fn conflicting_plural_forms_headers_for_one_locale_are_rejected() {
    let temp = TempDir::new().expect("tempdir");
    let first = temp.path().join("first.po");
    let second = temp.path().join("second.po");
    std::fs::write(
        &first,
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=(n != 1);\\n\"\n",
    )
    .expect("write first catalogue");
    std::fs::write(
        &second,
        "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=3; plural=(n == 0 ? 0 : 1);\\n\"\n",
    )
    .expect("write second catalogue");
    let error = DialogueCatalogProvider::load(vec![
        DialogueCatalogSource {
            locale: LocaleId::new("fr").expect("locale"),
            path: first,
        },
        DialogueCatalogSource {
            locale: LocaleId::new("fr").expect("locale"),
            path: second,
        },
    ])
    .expect_err("conflicting locale metadata is rejected");
    assert!(matches!(
        error,
        CliError::DialogueCatalogPluralFormsConflict { .. }
    ));
}

fn assert_plural_variant(
    provider: &DialogueCatalogProvider,
    locale: &LocaleId,
    id: &str,
    expected: &str,
    first_outcome: PluralResolutionOutcome,
) {
    let resolution = provider
        .resolve_plural(
            id,
            "one",
            "many",
            2,
            TextDomain::Line,
            locale,
            Some("formal"),
        )
        .expect("plural lookup");
    assert_eq!(resolution.template.as_deref(), Some(expected));
    assert_eq!(resolution.selected_arm, Some(2));
    assert_eq!(resolution.matched_locale.as_deref(), Some("fr"));
    let expected_context = format!("{id}&formal");
    assert_eq!(
        resolution.matched_context.as_deref(),
        Some(expected_context.as_str())
    );
    assert_eq!(resolution.attempts.len(), 2);
    assert_eq!(resolution.attempts[0].locale, "fr-CA");
    assert_eq!(resolution.attempts[0].context, format!("{id}&formal"));
    assert_eq!(resolution.attempts[0].selected_arm, Some(1));
    assert_eq!(resolution.attempts[0].outcome, first_outcome);
    assert_eq!(resolution.attempts[1].locale, "fr");
    assert_eq!(resolution.attempts[1].context, format!("{id}&formal"));
    assert_eq!(resolution.attempts[1].selected_arm, Some(2));
    assert_eq!(
        resolution.attempts[1].outcome,
        PluralResolutionOutcome::Matched
    );
}

fn source(locale: LocaleId, path: PathBuf) -> DialogueCatalogSource {
    DialogueCatalogSource { locale, path }
}

fn write_catalog(path: PathBuf, contents: String) -> PathBuf {
    std::fs::write(&path, contents).expect("write catalogue");
    path
}

fn catalog(header: String, entries: impl IntoIterator<Item = String>) -> String {
    let entries = entries.into_iter().collect::<Vec<_>>();
    format!("{header}\n{}\n", entries.join("\n\n"))
}

fn header(language: &str, plural_forms: Option<&str>) -> String {
    let plural_forms = plural_forms
        .map(|forms| format!("\"Plural-Forms: {forms}\\n\"\n"))
        .unwrap_or_default();
    format!("msgid \"\"\nmsgstr \"\"\n\"Language: {language}\\n\"\n{plural_forms}")
}

fn singular_entry(flags: Option<&str>, context: &str, source: &str, translation: &str) -> String {
    let flags = flags
        .map(|flags| format!("#, {flags}\n"))
        .unwrap_or_default();
    format!("{flags}msgctxt \"{context}\"\nmsgid \"{source}\"\nmsgstr \"{translation}\"")
}

fn plural_entry(
    flags: Option<&str>,
    context: &str,
    singular: &str,
    plural: &str,
    translations: &[&str],
) -> String {
    let flags = flags
        .map(|flags| format!("#, {flags}\n"))
        .unwrap_or_default();
    let translations = translations
        .iter()
        .enumerate()
        .map(|(arm, translation)| format!("msgstr[{arm}] \"{translation}\""))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{flags}msgctxt \"{context}\"\nmsgid \"{singular}\"\nmsgid_plural \"{plural}\"\n{translations}"
    )
}
