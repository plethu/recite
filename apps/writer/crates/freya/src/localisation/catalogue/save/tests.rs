use super::*;
impl Catalogue {
    pub fn save(&mut self, id: PoEntryId) -> Result<(), String> {
        self.save_entries(Some(&[id]))
    }
    pub(in crate::localisation) fn save_all(&mut self) -> Result<(), String> {
        self.save_entries(None)
    }
    fn save_entries(&mut self, ids: Option<&[PoEntryId]>) -> Result<(), String> {
        let mut request = self.begin_save(ids)?;
        let result = request
            .execute()
            .and_then(|saved| self.accept_save(&mut request, saved))
            .and_then(|ticket| {
                ticket
                    .map(|t| t.wait().map_err(|e| e.to_string()))
                    .transpose()
                    .map(|_| ())
            });
        self.finish_save();
        result
    }
}
const SOURCE: &str = "# translator note\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello {name}\"\nmsgstr \"Bonjour {name}\"\n";

#[test]
fn queue_cache_does_not_retain_an_obsolete_passage_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let catalogue = Catalogue::open(&path)?;
    let passages = {
        let example = recite_writer_model::WRITER_EXAMPLES[0].open()?;
        example.document().passage_snapshot()?
    };
    let old = Arc::downgrade(&passages);
    let first = catalogue.queue_index(Some(passages.clone()));
    assert!(std::rc::Rc::ptr_eq(
        &first,
        &catalogue.queue_index(Some(passages.clone()))
    ));
    drop(passages);
    assert!(old.upgrade().is_none());
    let replacement = catalogue.queue_index(None);
    assert!(!std::rc::Rc::ptr_eq(&first, &replacement));
    Ok(())
}

#[test]
fn local_save_patches_shared_queue_forms_without_rebuilding_structural_rows()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    let source = format!(
        "msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n{SOURCE}\n#~ msgid \"Obsolete\"\n#~ msgstr \"Old\"\n"
    );
    std::fs::write(&path, source)?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = catalogue
        .document
        .entries()
        .iter()
        .find(|e| e.context() == Some("11111111111111111111"))
        .ok_or("entry")?
        .id();
    let index = catalogue.queue_index(None);
    assert_eq!(
        index
            .borrow()
            .matching_entries(&catalogue, "bonjour", false),
        [id]
    );
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: false,
        },
    );
    let mut request = catalogue.begin_save(Some(&[id]))?;
    let saved = request.execute()?;
    let forms = saved.forms.as_ptr();
    catalogue.accept_save(&mut request, saved)?;
    catalogue.finish_save();
    assert!(std::rc::Rc::ptr_eq(&index, &catalogue.queue_index(None)));
    assert_eq!(
        index.borrow().matching_entries(&catalogue, "salut", false),
        [id]
    );
    assert!(
        index
            .borrow()
            .matching_entries(&catalogue, "bonjour", false)
            .is_empty()
    );
    assert!(!request.retired_forms.is_empty());
    assert_eq!(request.retired_forms.as_ptr(), forms);
    assert_eq!(request.retired_forms[0].1, ["bonjour {name}"]);
    catalogue.reload()?;
    assert!(!std::rc::Rc::ptr_eq(&index, &catalogue.queue_index(None)));
    Ok(())
}

#[test]
fn save_without_queue_returns_unused_normalized_forms_to_the_worker()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: false,
        },
    );
    let mut request = catalogue.begin_save(None)?;
    let saved = request.execute()?;
    let forms = saved.forms.as_ptr();
    assert!(catalogue.queue_cache.borrow().is_none());
    catalogue.accept_save(&mut request, saved)?;
    assert_eq!(request.retired_forms.as_ptr(), forms);
    assert_eq!(request.retired_forms[0].1, ["salut {name}"]);
    assert!(catalogue.queue_cache.borrow().is_none());
    Ok(())
}

#[test]
fn receipts_preserve_newer_drafts_and_reversions_and_block_replacement()
-> Result<(), Box<dyn std::error::Error>> {
    for text in ["Salut {name}", "Bonjour {name}", "Bonsoir {name}"] {
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("fr.po");
        std::fs::write(&path, SOURCE)?;
        let mut catalogue = Catalogue::open_recoverable(&path)?;
        let id = PoEntryId::new(0);
        let submitted = Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: false,
        };
        catalogue.update(id, submitted.clone());
        let mut request = catalogue.begin_save(Some(&[id]))?;
        assert!(Arc::ptr_eq(&request.document, &catalogue.document));
        assert!(Arc::ptr_eq(&request.drafts[&id], &catalogue.drafts[&id]));
        assert!(catalogue.begin_save(None).is_err());
        assert!(catalogue.reload().is_err());
        assert!(catalogue.accept_external(false, &request.baseline).is_err());
        catalogue.discard(id);
        assert!(catalogue.changed(id));
        let saved = std::thread::scope(|scope| scope.spawn(|| request.execute()).join().unwrap())?;
        // Repeating the submitted value retains its handle. Other text is newer.
        if text != submitted.forms[0] {
            let draft = if text == "Bonjour {name}" {
                catalogue.saved(id).unwrap()
            } else {
                Draft {
                    forms: vec![text.into()],
                    reviewed: true,
                }
            };
            catalogue.update(id, draft);
        }
        if let Some(ticket) = catalogue.accept_save(&mut request, saved)? {
            ticket.wait()?;
        }
        catalogue.finish_save();
        assert_eq!(
            catalogue.document.entry(id).unwrap().translation(),
            Some("Salut {name}")
        );
        assert_eq!(catalogue.draft(id).unwrap().forms, [text]);
        assert_eq!(catalogue.changed(id), text != "Salut {name}");
        drop(request);
        drop(catalogue);
        let reopened = Catalogue::open_recoverable(&path)?;
        assert_eq!(reopened.draft(id).unwrap().forms, [text]);
        assert_eq!(reopened.changed(id), text != "Salut {name}");
        assert!(std::fs::read_to_string(path)?.contains("# translator note"));
    }
    Ok(())
}
#[test]
fn failed_write_keeps_the_baseline_and_drafts() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let mut catalogue = Catalogue::open(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: true,
        },
    );
    let request = catalogue.begin_save(None)?;
    std::fs::write(&path, SOURCE.replace("Bonjour", "External"))?;
    let error = request.execute().err().ok_or("expected disk conflict")?;
    assert!(error.contains("changed on disk"));
    assert!(error.contains("Compare external changes"));
    assert!(!error.contains(&request.baseline.to_string()));
    catalogue.finish_save();
    assert!(Arc::ptr_eq(&catalogue.document, &request.document));
    assert_eq!(catalogue.saved_fingerprint(), &request.baseline);
    assert!(catalogue.changed(id));
    Ok(())
}

#[test]
fn recovery_failure_after_publication_keeps_the_new_baseline_and_can_retry()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(&path, SOURCE)?;
    let mut catalogue = Catalogue::open_recoverable(&path)?;
    let id = PoEntryId::new(0);
    catalogue.update(
        id,
        Draft {
            forms: vec!["Salut {name}".into()],
            reviewed: true,
        },
    );
    catalogue.flush_recovery()?;
    let mut request = catalogue.begin_save(None)?;
    let saved = request.execute()?;
    let sidecar = dir.path().join("fr.po.recite-editor-recovery.json");
    std::fs::remove_file(&sidecar)?;
    std::fs::create_dir(&sidecar)?;
    assert!(
        catalogue
            .accept_save(&mut request, saved)?
            .unwrap()
            .wait()
            .is_err()
    );
    catalogue.finish_save();
    assert!(!catalogue.changed(id));
    assert_eq!(
        catalogue.document.entry(id).unwrap().translation(),
        Some("Salut {name}")
    );
    assert_eq!(
        catalogue.saved_fingerprint(),
        &PoDocumentFingerprint::from_source(&std::fs::read_to_string(&path)?)
    );
    std::fs::remove_dir(sidecar)?;
    catalogue.save_all()?;
    assert!(catalogue.recovery_error().is_none());
    Ok(())
}

#[test]
fn a_partial_save_preserves_other_entries_including_newer_typing()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("fr.po");
    std::fs::write(
        &path,
        format!(
            "{SOURCE}\n{}",
            SOURCE.replace("11111111111111111111", "22222222222222222222")
        ),
    )?;
    let mut catalogue = Catalogue::open_recoverable(&path)?;
    let first = PoEntryId::new(0);
    let second = PoEntryId::new(1);
    let draft = |text: &str| Draft {
        forms: vec![text.into()],
        reviewed: false,
    };
    catalogue.update(first, draft("Salut {name}"));
    catalogue.update(second, draft("Bonsoir {name}"));
    let mut request = catalogue.begin_save(Some(&[first]))?;
    let saved = request.execute()?;
    catalogue.update(second, draft("Bonne nuit {name}"));
    if let Some(ticket) = catalogue.accept_save(&mut request, saved)? {
        ticket.wait()?;
    }
    catalogue.finish_save();
    assert!(!catalogue.changed(first));
    assert!(catalogue.changed(second));
    drop(catalogue);
    let reopened = Catalogue::open_recoverable(&path)?;
    assert_eq!(reopened.saved(first).unwrap().forms, ["Salut {name}"]);
    assert_eq!(reopened.saved(second).unwrap().forms, ["Bonjour {name}"]);
    assert_eq!(reopened.draft(second).unwrap().forms, ["Bonne nuit {name}"]);
    Ok(())
}
