use super::*;
use std::fs;

#[test]
fn post_publication_recovery_error_returns_an_applied_receipt_with_the_new_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("fr.po");
    let source = "msgctxt \"ffee6p8\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
    fs::write(&path, source)?;
    let mut catalogue = Catalogue::open_recoverable(&path)?;
    catalogue.update(
        PoEntryId::new(0),
        Draft {
            forms: vec!["Salut".into()],
            reviewed: true,
        },
    );
    catalogue.flush_recovery()?;
    // Inject a pending old snapshot without racing the asynchronous discard worker.
    catalogue.drafts.clear();
    let sidecar = directory.path().join("fr.po.recite-editor-recovery.json");
    fs::remove_file(&sidecar)?;
    fs::create_dir(&sidecar)?;
    let template = source.replace("Hello", "Welcome").replace("Bonjour", "");
    let preview = RefreshPreview::prepare(&catalogue, template.clone())?;
    let receipt = preview.apply(&mut catalogue, &template)?;
    assert!(receipt.recovery_error.is_some());
    assert!(catalogue.document.source().contains("Welcome"));
    assert_eq!(fs::read_to_string(&path)?, catalogue.document.source());
    assert!(!preview.is_current(&catalogue));
    fs::remove_dir(sidecar)?;
    catalogue.flush_recovery()?;
    assert!(catalogue.recovery_error().is_none());
    Ok(())
}
