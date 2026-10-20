use super::*;
#[test]
fn mode_is_a_verb_with_regional_english_spelling() -> Result<(), recite_ui::UiLocaleError> {
    assert_eq!(mode_label(&UiLocale::parse("en-US")?), "Localize");
    assert_eq!(mode_label(&UiLocale::parse("en-GB")?), "Localise");
    Ok(())
}
