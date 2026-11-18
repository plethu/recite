use super::Review;
use crate::test_support as support;

#[test]
fn preview_languages_filter_as_the_query_is_typed_and_source_only_clears_the_choice() {
    let directory = support::project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n").unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.trial.settings.language_page = 3;
    writer.set_preview_language_query("fr".into());
    let french = writer.trial.settings.languages.len();
    assert!(french > 0);
    assert_eq!(writer.trial.settings.query, "fr");
    assert_eq!(writer.trial.settings.language_page, 0);
    writer.set_preview_language_query("fr-CA".into());
    assert!(
        writer
            .trial
            .settings
            .languages
            .iter()
            .any(|choice| choice.value == "fr-CA")
    );
    writer.set_preview_language_query(String::new());
    assert!(writer.trial.settings.languages.is_empty());
    writer.set_preview_language_query("fr-CA".into());
    writer.choose_preview_language("fr-CA".into());
    assert!(!writer.previews_source_only());
    writer.set_preview_language_query("fr-CA".into());
    writer.set_source_language_only(true);
    assert!(writer.previews_source_only());
    assert!(writer.trial.settings.locale.is_empty());
    assert!(writer.trial.settings.languages.is_empty());
    assert!(writer.trial.settings.query.is_empty());
}

#[test]
fn turning_source_only_off_restores_the_last_language_or_opens_the_language_search() {
    let directory = support::project(":: start default\n> @ffee6p8\n  Hello.\n-> END\n").unwrap();
    let mut writer = support::open(&directory).unwrap();
    assert!(writer.previews_source_only());
    writer.set_source_language_only(false);
    assert!(!writer.previews_source_only());
    assert!(writer.trial.settings.locale.is_empty());
    assert!(writer.trial.settings.choosing_language);
    assert!(writer.attention.pane_request().is_some());
    assert!(matches!(writer.trial.settings.review, Review::Inputs));
    writer.choose_preview_language("fr-CA".into());
    assert!(!writer.trial.settings.choosing_language);
    writer.set_source_language_only(true);
    assert!(writer.previews_source_only());
    assert_eq!(writer.trial.settings.capture().locale, "");
    writer.set_source_language_only(false);
    assert_eq!(writer.trial.settings.locale, "fr-CA");
    assert!(!writer.trial.settings.choosing_language);
}
