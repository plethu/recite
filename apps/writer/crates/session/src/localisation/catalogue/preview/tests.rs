use super::*;
const SOURCE: &str = "msgctxt \"ffee6p8\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n";
use crate::preview::{capture_provenance, catalogue_stale};

#[test]
fn catalogue_mutations_invalidate_localised_trials_only() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let capture = |catalogue: &Catalogue| capture_provenance(String::new(), true, Some(catalogue));
    let source = capture_provenance(String::new(), false, Some(&catalogue));
    let id = PoEntryId::new(0);
    let saved = capture(&catalogue);
    let running = catalogue.preview_input(true).prepare()?;
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: true,
        },
    );
    assert!(catalogue_stale(&saved, Some(&catalogue)));
    assert!(running.source().contains("Bonjour {name}"));
    assert!(
        catalogue
            .preview_input(true)
            .prepare()?
            .source()
            .contains("Salut {name}")
    );
    let draft = capture(&catalogue);
    assert!(!catalogue_stale(&draft, Some(&catalogue)));
    catalogue.save(id)?;
    assert!(catalogue_stale(&draft, Some(&catalogue)));
    let saved = capture(&catalogue);
    catalogue.reload()?;
    assert!(catalogue_stale(&saved, Some(&catalogue)));
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
    assert!(catalogue_stale(&edited, Some(&catalogue)));
    assert!(catalogue_stale(&reloaded, Some(&catalogue)));
    assert!(!catalogue_stale(&source, Some(&catalogue)));
    assert!(catalogue_stale(&capture(&catalogue), None));
    assert!(catalogue_stale(
        &capture_provenance(String::new(), true, None),
        Some(&catalogue)
    ));
    assert!(!catalogue_stale(&source, None));
    Ok(())
}
