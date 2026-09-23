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
            forms: vec!["Bonjour".into()],
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
            forms: vec!["Bonjour".into()],
            reviewed: true,
        },
    );
    assert!(catalogue.save(id).is_err());
    assert!(catalogue.dirty());
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
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
            forms: vec!["Salut {name}".into()],
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
        catalogue.draft(id).map(|d| d.forms.join("")),
        Some("Salut {name}".into())
    );
    catalogue.discard(id);
    catalogue.reload()?;
    assert_eq!(
        catalogue.draft(id).map(|d| d.forms.join("")),
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
            forms: vec!["Local {name}".into()],
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
    assert!(
        comparison
            .rows
            .iter()
            .any(|row| row.after.as_deref() == Some("External {name}"))
    );
    catalogue.accept_external(true, &comparison.fingerprint)?;
    assert_eq!(
        catalogue.draft(id).map(|d| d.forms.join("")),
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
            forms: vec!["Draft".into()],
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

#[test]
fn plural_review_is_atomic_and_variant_drafts_are_independent() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    let source = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\n#, fuzzy\nmsgctxt \"22222222222222222222\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n\n#, fuzzy\nmsgctxt \"22222222222222222222&formal\"\nmsgid \"{count} ticket\"\nmsgid_plural \"{count} tickets\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"\n";
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = catalogue
        .entry_for("22222222222222222222", "{count} ticket")
        .ok_or("default")?;
    let formal = catalogue
        .entry_for("22222222222222222222&formal", "{count} ticket")
        .ok_or("formal")?;
    catalogue.update(
        formal,
        Draft {
            forms: vec!["Formal {count}".into(), "Formal {count}".into()],
            reviewed: false,
        },
    );
    catalogue.update(
        id,
        Draft {
            forms: vec!["{count} billet".into(), "missing placeholder".into()],
            reviewed: true,
        },
    );
    assert!(catalogue.save(id).is_err());
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        source
    );
    assert!(catalogue.changed(id));
    catalogue.update(
        id,
        Draft {
            forms: vec!["{count} billet".into(), "{count} billets".into()],
            reviewed: true,
        },
    );
    catalogue.save(id)?;
    assert!(catalogue.changed(formal));
    let saved = Catalogue::open(&path)?;
    assert_eq!(
        saved.draft(id).ok_or("saved")?.forms,
        ["{count} billet", "{count} billets"]
    );
    assert!(saved.draft(id).ok_or("saved")?.reviewed);
    assert_eq!(saved.draft(formal).ok_or("formal")?.forms, ["", ""]);
    Ok(())
}

#[test]
fn choosing_the_disk_catalogue_discards_plural_drafts() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    let source = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"ticket\"\nmsgid_plural \"22222222222222222222\"\nmsgstr[0] \"billet\"\nmsgstr[1] \"billets\"\n";
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = catalogue
        .entry_for("22222222222222222222", "ticket")
        .ok_or("entry")?;
    catalogue.update(
        id,
        Draft {
            forms: vec!["one draft".into(), "many drafts".into()],
            reviewed: true,
        },
    );
    std::fs::write(&path, source.replace("billet", "titre")).map_err(|e| e.to_string())?;
    let comparison = catalogue.compare()?;
    assert_eq!(comparison.rows.len(), 2);
    catalogue.accept_external(false, &comparison.fingerprint)?;
    let draft = catalogue.draft(id).ok_or("draft")?;
    assert_eq!(draft.forms, ["titre", "titres"]);
    assert!(draft.reviewed);
    assert!(!catalogue.dirty());
    assert!(!Catalogue::open(&path)?.dirty());
    Ok(())
}

#[test]
fn changed_plural_semantics_cannot_silently_rebase_a_draft() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    let source = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"ticket\"\nmsgid_plural \"tickets\"\nmsgstr[0] \"billet\"\nmsgstr[1] \"billets\"\n";
    std::fs::write(&path, source).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = catalogue
        .entry_for("22222222222222222222", "ticket")
        .ok_or("entry")?;
    let draft = Draft {
        forms: vec!["one draft".into(), "many drafts".into()],
        reviewed: false,
    };
    catalogue.update(id, draft.clone());
    for changed in [
        source.replace("n > 1", "n != 1"),
        source.replace("msgid_plural \"tickets\"", "msgid_plural \"passes\""),
    ] {
        std::fs::write(&path, &changed).map_err(|e| e.to_string())?;
        let comparison = catalogue.compare()?;
        assert!(
            catalogue
                .accept_external(true, &comparison.fingerprint)
                .is_err()
        );
        assert_eq!(catalogue.draft(id), Some(draft.clone()));
        assert_eq!(
            std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
            changed
        );
    }
    Ok(())
}

#[test]
fn trial_snapshots_neither_save_nor_review_and_reject_stale_drafts() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: false,
        },
    );
    assert_eq!(
        catalogue
            .preview_document(false)?
            .entry(id)
            .and_then(|e| e.translation()),
        Some("Bonjour {name}")
    );
    let snapshot = catalogue.preview_document(true)?;
    assert_eq!(
        snapshot.entry(id).and_then(|e| e.translation()),
        Some("Salut {name}")
    );
    assert!(catalogue.dirty());
    assert!(!catalogue.draft(id).ok_or("draft")?.reviewed);
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        SOURCE
    );
    std::fs::write(&path, SOURCE.replace("Bonjour", "Bonsoir")).map_err(|e| e.to_string())?;
    assert!(catalogue.preview_document(true).is_err());
    assert_eq!(
        catalogue
            .preview_document(false)?
            .entry(id)
            .and_then(|e| e.translation()),
        Some("Bonsoir {name}")
    );
    assert_eq!(
        snapshot.entry(id).and_then(|e| e.translation()),
        Some("Salut {name}")
    );
    Ok(())
}

#[test]
fn save_all_is_atomic_and_keeps_every_draft_on_validation_or_disk_conflict() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    let source = format!(
        "{SOURCE}\nmsgctxt \"22222222222222222222\"\nmsgid \"Bye {{name}}\"\nmsgstr \"Au revoir {{name}}\"\n"
    );
    std::fs::write(&path, &source).map_err(|e| e.to_string())?;
    let mut catalogue = Catalogue::open(&path)?;
    let first = PoEntryId::new(0);
    let second = PoEntryId::new(1);
    catalogue.update(
        first,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: true,
        },
    );
    catalogue.update(
        second,
        Draft {
            forms: vec!["Au revoir".into()],
            reviewed: true,
        },
    );
    assert!(catalogue.save_all().is_err());
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        source
    );
    assert!(catalogue.changed(first) && catalogue.changed(second));
    catalogue.update(
        second,
        Draft {
            forms: vec!["Adieu {name}".into()],
            reviewed: true,
        },
    );
    let external = format!("# external edit\n{source}");
    std::fs::write(&path, &external).map_err(|e| e.to_string())?;
    assert!(catalogue.save_all().is_err());
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        external
    );
    assert!(catalogue.changed(first) && catalogue.changed(second));
    std::fs::write(&path, &source).map_err(|e| e.to_string())?;
    catalogue.save_all()?;
    assert!(!catalogue.dirty());
    let saved = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    assert!(saved.contains("Salut {name}") && saved.contains("Adieu {name}"));
    Ok(())
}

#[test]
fn recovery_restores_drafts_without_overwriting_changed_catalogue() -> Result<(), String> {
    let dir = tempfile::tempdir().map_err(|e| e.to_string())?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE).map_err(|e| e.to_string())?;
    let id = PoEntryId::new(0);
    let mut original = Catalogue::open_recoverable(&path)?;
    assert!(Catalogue::open_recoverable(&path).is_err());
    original.update(
        id,
        Draft {
            forms: vec!["Recovered {name}".into()],
            reviewed: false,
        },
    );
    original.flush_recovery()?;
    drop(original);
    let changed = format!("# external\n{SOURCE}");
    std::fs::write(&path, &changed).map_err(|e| e.to_string())?;
    let mut restored = Catalogue::open_recoverable(&path)?;
    assert_eq!(
        restored.draft(id).ok_or("draft")?.forms,
        ["Recovered {name}"]
    );
    assert!(restored.save_all().is_err());
    assert_eq!(
        std::fs::read_to_string(&path).map_err(|e| e.to_string())?,
        changed
    );
    restored.discard(id);
    restored.flush_recovery()?;
    drop(restored);
    assert!(!Catalogue::open_recoverable(&path)?.dirty());
    Ok(())
}

#[test]
fn plural_header_case_does_not_block_reviewed_edits() -> Result<(), Box<dyn std::error::Error>> {
    for header in ["Plural-Forms", "plural-forms", "pLuRaL-fOrMs"] {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("fr.po");
        let source = format!(
            "msgid \"\"\nmsgstr \"Language: fr\\n{header}: nplurals=2; plural=(n > 1);\\n\"\n\n#, fuzzy\nmsgctxt \"22222222222222222222\"\nmsgid \"ticket\"\nmsgid_plural \"tickets\"\nmsgstr[0] \"billet\"\nmsgstr[1] \"billets\"\n"
        );
        std::fs::write(&path, source)?;
        let mut catalogue = Catalogue::open(&path)?;
        let id = catalogue
            .entry_for("22222222222222222222", "ticket")
            .ok_or("entry")?;
        assert_eq!(catalogue.plural_rule(), Some("nplurals=2; plural=(n > 1);"));
        catalogue.update(
            id,
            Draft {
                forms: vec!["ticket traduit".into(), "tickets traduits".into()],
                reviewed: true,
            },
        );
        catalogue.save(id)?;
        let reopened = Catalogue::open(&path)?;
        let draft = reopened.draft(id).ok_or("saved entry")?;
        assert_eq!(draft.forms, ["ticket traduit", "tickets traduits"]);
        assert!(draft.reviewed);
        assert!(std::fs::read_to_string(&path)?.contains(&format!("{header}:")));
    }
    Ok(())
}

#[test]
fn discarding_external_conflicts_accepts_removed_entries_and_clears_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    for changed in [
        SOURCE.replace("Hello {name}", "Goodbye {name}"),
        "msgid \"\"\nmsgstr \"Language: fr\\n\"\n".into(),
    ] {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("fr.po");
        std::fs::write(&path, SOURCE)?;
        let mut catalogue = Catalogue::open_recoverable(&path)?;
        catalogue.update(
            PoEntryId::new(0),
            Draft {
                forms: vec!["Draft {name}".into()],
                reviewed: true,
            },
        );
        catalogue.flush_recovery()?;
        std::fs::write(&path, &changed)?;
        let comparison = catalogue.compare()?;
        std::fs::write(&path, format!("# newer\n{changed}"))?;
        assert!(
            catalogue
                .accept_external(false, &comparison.fingerprint)
                .is_err()
        );
        assert!(catalogue.dirty());
        let comparison = catalogue.compare()?;
        catalogue.accept_external(false, &comparison.fingerprint)?;
        assert!(!catalogue.dirty());
        assert_eq!(catalogue.document.source(), format!("# newer\n{changed}"));
        drop(catalogue);
        assert!(!Catalogue::open_recoverable(&path)?.dirty());
    }
    Ok(())
}
