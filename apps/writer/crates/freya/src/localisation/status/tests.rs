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
            text: "Bonjour".into(),
            reviewed: true,
        },
    );
    let status = TranslationStatus::for_entry(&catalogue, id);
    assert_eq!(status, TranslationStatus::ReviewPending);
    assert!(status.needs_attention());
    catalogue.save(id)?;
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::Reviewed
    );
    assert!(!TranslationStatus::for_entry(&catalogue, id).needs_attention());
    catalogue.update(
        id,
        Draft {
            text: "  ".into(),
            reviewed: false,
        },
    );
    assert_eq!(
        TranslationStatus::for_entry(&catalogue, id),
        TranslationStatus::Untranslated
    );
    Ok(())
}
