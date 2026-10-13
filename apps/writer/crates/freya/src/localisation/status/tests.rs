use super::super::catalogue::Draft;
use super::*;

#[test]
fn an_unsaved_review_remains_in_attention_until_saved() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "#, fuzzy\nmsgctxt \"line@11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n",
    )?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = catalogue
        .entry_for("line@11111111111111111111", "Hello")
        .ok_or("entry")?;
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::NeedsReview
    );
    catalogue.update(
        id,
        Draft {
            forms: vec!["Bonjour".into()],
            reviewed: true,
        },
    );
    let status = TranslationStatus::for_entry(&catalogue, id);
    assert_eq!(status, TranslationStatus::ReviewPending);
    assert!(status.needs_attention());
    assert!(TranslationStatus::entry_label(&catalogue, id).contains("Unsaved changes"));
    catalogue.save(id)?;
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::Reviewed
    );
    assert!(!TranslationStatus::for_entry(&catalogue, id).needs_attention());
    assert!(!TranslationStatus::entry_label(&catalogue, id).contains("Unsaved changes"));
    catalogue.update(
        id,
        Draft {
            forms: vec!["  ".into()],
            reviewed: false,
        },
    );
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::Untranslated
    );
    Ok(())
}

#[test]
fn fuzzy_singular_entry_without_translation_stays_untranslated()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        "#, fuzzy\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello\"\n",
    )?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::Untranslated
    );
    catalogue.update(
        id,
        Draft {
            forms: vec!["Draft".into()],
            reviewed: false,
        },
    );
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::NeedsReview
    );
    Ok(())
}
