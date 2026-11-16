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
    writer.trial.settings.locale = "fr-CA".into();
    writer.use_source_language_only();
    assert!(writer.trial.settings.locale.is_empty());
    assert!(writer.trial.settings.languages.is_empty());
    assert!(writer.trial.settings.query.is_empty());
}
