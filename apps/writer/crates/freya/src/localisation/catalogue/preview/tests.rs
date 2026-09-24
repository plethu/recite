use super::*;
const SOURCE: &str =
    "msgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n";
use crate::preview_panel::Snapshot;

#[test]
fn catalogue_mutations_invalidate_localised_trials_only() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let capture = |catalogue: &Catalogue| Snapshot::capture(String::new(), true, Some(catalogue));
    let source = Snapshot::capture(String::new(), false, Some(&catalogue));
    let id = PoEntryId::new(0);
    let saved = capture(&catalogue);
    let running = catalogue.preview_document(true)?;
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: true,
        },
    );
    assert!(saved.stale(Some(&catalogue)));
    assert!(running.source().contains("Bonjour {name}"));
    assert!(
        catalogue
            .preview_document(true)?
            .source()
            .contains("Salut {name}")
    );
    let draft = capture(&catalogue);
    assert!(!draft.stale(Some(&catalogue)));
    catalogue.save(id)?;
    assert!(draft.stale(Some(&catalogue)));
    let saved = capture(&catalogue);
    catalogue.reload()?;
    assert!(saved.stale(Some(&catalogue)));
    let reloaded = capture(&catalogue);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Bonsoir {name}".into()],
            reviewed: true,
        },
    );
    let edited = capture(&catalogue);
    catalogue.discard(id);
    assert!(edited.stale(Some(&catalogue)));
    assert!(reloaded.stale(Some(&catalogue)));
    assert!(!source.stale(Some(&catalogue)));
    assert!(capture(&catalogue).stale(None));
    assert!(Snapshot::capture(String::new(), true, None).stale(Some(&catalogue)));
    assert!(!source.stale(None));
    Ok(())
}
