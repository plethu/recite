use super::*;
use crate::localisation::tests::{fixture, form, key};
use recite_core::po::PoDocument;
use std::fs;

#[test]
fn caught_panic_closes_using_the_last_admitted_recovery_without_serializing_uncertain_state() {
    let (directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Recover this draft.".into());
    writer
        .localisation
        .slot
        .owner_mut()
        .unwrap()
        .flush_recovery()
        .unwrap();
    let recovery = directory.path().join("fr.po.recite-editor-recovery.json");
    let before = fs::read(&recovery).unwrap();
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    let mut completed = Checked::new(checkout, Action::Query, Query::default(), None).execute_with(
        |checkout, _| {
            let owner = checkout.owner.as_mut().unwrap();
            owner.catalogue.document = Arc::new(
                PoDocument::parse("msgctxt \"11111111111111111111\"\nmsgid \"Uncertain replacement\"\nmsgstr \"\"\n").unwrap(),
            );
            panic!("injected after partial mutation");
        },
    );
    writer.accept_checked_catalogue(&mut completed).unwrap();
    assert!(writer.localisation.slot.read_only());
    assert!(writer.localisation.slot.owner_mut().is_none());
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    writer.phase = Phase::Closing;
    let mut completed =
        Checked::new(checkout, Action::FlushForClose, Query::default(), None).execute();
    writer.accept_checked_catalogue(&mut completed).unwrap();
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
    assert_eq!(fs::read(&recovery).unwrap(), before);
    drop(writer);
    let restored = Catalogue::open_recoverable(&directory.path().join("fr.po")).unwrap();
    assert_eq!(
        restored.draft(key.id).unwrap().forms,
        ["Recover this draft."]
    );
    assert!(!restored.document.source().contains("Uncertain replacement"));
}

#[test]
fn failed_close_flush_restores_the_owner_and_resolves_the_close_prompt() {
    let (_directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Keep the owner.".into());
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    writer.phase = Phase::Closing;
    let mut completed = Checked::new(checkout, Action::FlushForClose, Query::default(), None)
        .not_started("worker unavailable".into());
    writer.accept_checked_catalogue(&mut completed).unwrap();
    assert!(writer.phase == Phase::ConfirmClose);
    assert!(!writer.localisation.working());
    assert_eq!(form(&writer, &key), "Keep the owner.");
    assert!(writer.status.contains("worker unavailable"));
}

#[test]
fn cancellation_returns_the_exact_catalogue_owner_and_frozen_draft() {
    let (_directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Keep on cancellation.".into());
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    checkout.ticket.cancel();
    let mut completed = Checked::new(checkout, Action::Query, Query::default(), None).execute();
    writer.accept_checked_catalogue(&mut completed).unwrap();
    assert!(writer.localisation.slot.identity() == Some(&key.catalogue));
    assert_eq!(form(&writer, &key), "Keep on cancellation.");
    assert!(!writer.localisation.slot.read_only());
}

#[test]
fn failed_recovery_preflight_prevents_exclusive_mutation_and_retains_newer_drafts() {
    let (directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Previously admitted.".into());
    writer
        .localisation
        .slot
        .owner_mut()
        .unwrap()
        .flush_recovery()
        .unwrap();
    let recovery = directory.path().join("fr.po.recite-editor-recovery.json");
    fs::remove_file(&recovery).unwrap();
    fs::create_dir(&recovery).unwrap();
    writer.update_translation(&key, 0, "Newer undurable draft.".into());
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    let mut performed = false;
    let mut completed =
        Checked::new(checkout, Action::Query, Query::default(), None).execute_with(|_, _| {
            performed = true;
            Ok(())
        });
    assert!(!performed);
    writer.accept_checked_catalogue(&mut completed).unwrap();
    assert!(!writer.localisation.slot.read_only());
    assert_eq!(form(&writer, &key), "Newer undurable draft.");
    fs::remove_dir(&recovery).unwrap();
    let checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    let mut completed = Checked::new(checkout, Action::Query, Query::default(), None).execute_with(
        |checkout, _| {
            checkout.owner.as_mut().unwrap().catalogue.document = Arc::new(
                PoDocument::parse(
                    "msgctxt \"11111111111111111111\"\nmsgid \"Uncertain\"\nmsgstr \"\"\n",
                )
                .unwrap(),
            );
            panic!("after preflight repaired durability");
        },
    );
    writer.accept_checked_catalogue(&mut completed).unwrap();
    let snapshot = fs::read_to_string(&recovery).unwrap();
    assert!(snapshot.contains("Newer undurable draft."));
    assert!(!snapshot.contains("Uncertain"));
}
