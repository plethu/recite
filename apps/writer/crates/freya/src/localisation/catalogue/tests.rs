use super::*;
const SOURCE: &str = "# translator note\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n";

#[test]
fn save_draft_preserves_comments_and_review_revalidates_placeholders() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            text: "Bonjour".into(),
            reviewed: false,
        },
    );
    catalogue.save(id)?;
    assert!(!catalogue.dirty());
    assert!(catalogue.document.source().contains("# translator note"));
    assert!(
        catalogue
            .document
            .entry(id)
            .is_some_and(|e| e.flags().iter().any(|f| f == "fuzzy"))
    );
    catalogue.update(
        id,
        Draft {
            text: "Bonjour".into(),
            reviewed: true,
        },
    );
    assert!(catalogue.save(id).is_err());
    assert!(catalogue.dirty());
    catalogue.update(
        id,
        Draft {
            text: "Salut {name}".into(),
            reviewed: true,
        },
    );
    catalogue.save(id)?;
    let disk = PoDocument::read(path).map_err(|e| e.to_string())?;
    assert!(
        disk.entry(id)
            .is_some_and(|e| e.flags().is_empty() && e.translation() == Some("Salut {name}"))
    );
    Ok(())
}
#[test]
fn external_changes_and_failed_reload_preserve_both_versions() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            text: "Salut {name}".into(),
            reviewed: false,
        },
    );
    let external = SOURCE.replace("Bonjour", "Bonsoir");
    std::fs::write(&path, &external).map_err(|e| e.to_string())?;
    assert!(catalogue.save(id).is_err());
    assert!(catalogue.reload().is_err());
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        external
    );
    assert_eq!(
        catalogue.draft(id).map(|d| d.text),
        Some("Salut {name}".into())
    );
    catalogue.discard(id);
    catalogue.reload()?;
    assert_eq!(
        catalogue.draft(id).map(|d| d.text),
        Some("Bonsoir {name}".into())
    );
    assert!(
        catalogue
            .entry_for("11111111111111111111", "Revised source")
            .is_none()
    );
    Ok(())
}

#[test]
fn comparison_is_bound_to_the_seen_external_version() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            text: "Local {name}".into(),
            reviewed: false,
        },
    );
    let comparison = catalogue.compare()?;
    std::fs::write(&path, SOURCE.replace("Bonjour", "External")).map_err(|e| e.to_string())?;
    assert!(
        catalogue
            .accept_external(true, &comparison.fingerprint)
            .is_err()
    );
    assert!(catalogue.dirty());
    let comparison = catalogue.compare()?;
    assert!(comparison.text.contains("External {name}"));
    catalogue.accept_external(true, &comparison.fingerprint)?;
    assert_eq!(
        catalogue.draft(id).map(|d| d.text),
        Some("Local {name}".into())
    );
    catalogue.save(id)?;
    assert!(
        std::fs::read_to_string(path)
            .map_err(|e| e.to_string())?
            .contains("Local {name}")
    );
    Ok(())
}

#[test]
fn refresh_refuses_external_changes_and_unsaved_drafts() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    let source = "msgctxt \"11111111111111111111\"\nmsgid \"Hello\"\nmsgstr \"Bonjour\"\n";
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let baseline = catalogue.document.fingerprint();
    let template =
        PoDocument::parse(source.replace("Hello", "Hello again")).map_err(|e| e.to_string())?;
    let refreshed = catalogue
        .document
        .refreshed(&template)
        .map_err(|e| e.to_string())?;
    let id = catalogue.document.entries()[0].id();
    catalogue.update(
        id,
        Draft {
            text: "Draft".into(),
            reviewed: false,
        },
    );
    assert!(
        catalogue
            .replace_refreshed(refreshed.clone(), &baseline)
            .is_err()
    );
    catalogue.discard(id);
    std::fs::write(&path, source.replace("Bonjour", "External")).map_err(|e| e.to_string())?;
    assert!(catalogue.replace_refreshed(refreshed, &baseline).is_err());
    assert!(
        std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())?
            .contains("External")
    );
    assert_eq!(catalogue.document.fingerprint(), baseline);
    Ok(())
}
