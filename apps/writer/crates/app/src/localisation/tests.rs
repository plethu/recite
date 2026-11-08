// Fixtures here are shared with frontend tests through the `test-support` feature.
#![allow(unused_imports, dead_code, clippy::unwrap_used, clippy::expect_used)]
pub use super::*;
pub use crate::{test_support as support, workspaces::tests::ToolDriver};
pub use recite_writer_model::View;
pub use recite_writer_session::localisation::catalogue::Catalogue;
pub use std::fs;
pub use tokio::sync::mpsc;
#[path = "replacement/tests.rs"]
pub mod replacement;

pub const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> END\n";
pub const PO: &str = "#. file: scene.recite\n#. block: start\n# translator note\nmsgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n";
pub fn fixture() -> (tempfile::TempDir, Writer, ToolDriver) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(directory.path().join("fr.po"), PO).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut driver = ToolDriver::new(&mut writer);
    writer.open_localisation();
    writer.localisation.path = directory.path().join("fr.po").display().to_string();
    writer.open_catalogue_path();
    assert!(writer.localisation.working());
    driver.finish(&mut writer);
    assert_eq!(writer.localisation.queue.total, 1, "{}", writer.status);
    let key = EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id: writer.localisation.queue.rows[0].id,
    };
    writer.inspect_translation(&key);
    driver.finish(&mut writer);
    (directory, writer, driver)
}
pub fn key(writer: &Writer) -> EntryKey {
    writer.localisation.entry.as_ref().unwrap().key.clone()
}
pub fn form(writer: &Writer, key: &EntryKey) -> String {
    writer.localisation.slot.draft(key.id).unwrap().forms[0].clone()
}
#[test]
pub fn catalogue_save_retains_newer_typing_and_recovery_and_preserves_po_metadata() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Salut.".into());
    writer.save_translation_drafts();
    assert!(writer.localisation.saving());
    writer.request_close();
    assert!(writer.phase == Phase::Writing);
    driver.complete_after(&mut writer, |writer| {
        writer.update_translation(&key, 0, "Newer translation.".into())
    });
    assert!(!writer.localisation.saving());
    assert_eq!(form(&writer, &key), "Newer translation.");
    let saved = fs::read_to_string(directory.path().join("fr.po")).unwrap();
    assert!(saved.contains("Salut.") && saved.contains("# translator note"));
    writer.request_close();
    assert!(writer.phase == Phase::ConfirmClose);
    writer.close_with_recovery();
    assert!(writer.phase == Phase::Closing);
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
    drop(writer);
    let reopened = Catalogue::open_recoverable(&directory.path().join("fr.po")).unwrap();
    assert_eq!(
        reopened.draft(key.id).unwrap().forms,
        ["Newer translation."]
    );
}
#[test]
pub fn save_all_then_close_publishes_translations_and_source_through_the_normal_source_worker() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Salut.".into());
    writer
        .session
        .edit(|model| {
            model.select(View::Passage("ffee6p8".into()))?;
            model.set_draft("Source edit.".into());
            Ok(())
        })
        .unwrap();
    let (sender, mut source_jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.request_close();
    writer.save_and_close();
    driver.finish(&mut writer);
    assert!(writer.work.busy());
    support::finish_work(&mut writer, &mut source_jobs).unwrap();
    assert!(writer.phase == Phase::Closing);
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
    assert!(
        fs::read_to_string(directory.path().join("fr.po"))
            .unwrap()
            .contains("Salut.")
    );
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Source edit.")
    );
}
#[test]
pub fn cancelling_close_intent_during_the_recovery_flush_keeps_writer_open() {
    let (_directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Keep this translation.".into());
    writer.request_close();
    writer.close_with_recovery();
    driver.complete_after(&mut writer, |writer| {
        writer.phase = Phase::Writing;
        writer.localisation.keep_editing();
        writer.work.keep_editing();
    });
    assert!(writer.phase == Phase::Writing);
    assert_eq!(form(&writer, &key), "Keep this translation.");
}
#[test]
pub fn a_new_catalogue_lifetime_rejects_queued_field_actions_from_the_previous_owner() {
    let (_directory, mut writer, mut driver) = fixture();
    let old = key(&writer);
    writer.catalogue_action(work::Action::Reload);
    driver.finish(&mut writer);
    writer.update_translation(&old, 0, "Must not replace reloaded text.".into());
    assert_eq!(form(&writer, &old), "Bonjour.");
    assert!(!writer.localisation.dirty());
    assert!(writer.localisation.slot.identity() != Some(&old.catalogue));
}

#[test]
pub fn save_all_cannot_skip_translation_drafts_while_a_query_owns_the_catalogue() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Saved together.".into());
    let (sender, mut source_jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.search_catalogue();
    assert!(writer.localisation.working());
    assert!(writer.localisation.dirty());
    assert!(writer.localisation.needs_save());
    writer.save_all();
    assert!(!writer.work.busy());
    assert!(source_jobs.try_recv().is_err());
    assert!(writer.status.contains("catalogue action"));
    driver.finish(&mut writer);
    assert!(writer.localisation.dirty());
    writer.save_all();
    assert!(writer.localisation.saving());
    driver.finish(&mut writer);
    support::finish_work(&mut writer, &mut source_jobs).unwrap();
    assert!(!writer.localisation.dirty());
    assert!(
        fs::read_to_string(directory.path().join("fr.po"))
            .unwrap()
            .contains("Saved together.")
    );
}

#[test]
pub fn newer_translation_typing_during_save_all_and_close_keeps_writer_open() {
    let (_directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Captured translation.".into());
    let (sender, mut source_jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.request_close();
    writer.save_and_close();
    driver.complete_after(&mut writer, |writer| {
        writer.phase = Phase::Writing;
        writer.update_translation(&key, 0, "Newer translation.".into());
    });
    support::finish_work(&mut writer, &mut source_jobs).unwrap();
    assert!(writer.phase == Phase::ConfirmClose);
    assert_eq!(form(&writer, &key), "Newer translation.");
    assert!(writer.localisation.dirty());
}

#[test]
pub fn keep_editing_cancels_close_continuation_during_translation_save() {
    let (_directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Saved translation.".into());
    let (sender, mut source_jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.request_close();
    writer.save_and_close();
    driver.complete_after(&mut writer, |writer| {
        writer.phase = Phase::Writing;
        writer.localisation.keep_editing();
        writer.work.keep_editing();
    });
    support::finish_work(&mut writer, &mut source_jobs).unwrap();
    assert!(writer.phase == Phase::Writing);
    assert!(!writer.localisation.dirty());
}

#[test]
pub fn catalogue_disk_resolution_rechecks_local_drafts_and_preserves_external_metadata() {
    let (directory, mut writer, mut driver) = fixture();
    let old = key(&writer);
    writer.update_translation(&old, 0, "Local draft.".into());
    writer.review_translation(&old, true);
    let disk = PO
        .replace("Bonjour.", "External translation.")
        .replace("# translator note", "# external translator note");
    let path = directory.path().join("fr.po");
    fs::write(&path, &disk).unwrap();
    writer.compare_catalogue();
    driver.finish(&mut writer);
    assert!(matches!(
        writer.localisation.destination,
        Destination::Comparison
    ));
    assert!(
        writer
            .localisation
            .comparison
            .as_ref()
            .unwrap()
            .changes
            .iter()
            .any(|row| row.after.as_deref() == Some("External translation."))
    );
    // A changed owner invalidates the already captured comparison even without a field callback.
    let catalogue = &mut writer.localisation.slot.owner_mut().unwrap().catalogue;
    let mut draft = catalogue.draft(old.id).unwrap();
    draft.forms[0] = "Newer draft.".into();
    catalogue.update(old.id, draft);
    writer.resolve_catalogue(ExternalDecision::UseDisk);
    driver.finish(&mut writer);
    assert_eq!(form(&writer, &old), "Newer draft.");
    assert!(writer.status.contains("changed since comparison"));
    writer.compare_catalogue();
    driver.finish(&mut writer);
    writer.resolve_catalogue(ExternalDecision::KeepDrafts);
    driver.finish(&mut writer);
    assert!(matches!(
        writer.localisation.destination,
        Destination::Queue
    ));
    assert!(writer.localisation.slot.identity() != Some(&old.catalogue));
    assert_eq!(fs::read_to_string(&path).unwrap(), disk);
    assert_eq!(form(&writer, &old), "Newer draft.");
    assert!(!writer.localisation.slot.draft(old.id).unwrap().reviewed);
    writer.update_translation(&old, 0, "Stale field must not overwrite.".into());
    assert_eq!(form(&writer, &old), "Newer draft.");
    writer.save_translation_drafts();
    driver.finish(&mut writer);
    let saved = fs::read_to_string(&path).unwrap();
    assert!(saved.contains("Newer draft.") && saved.contains("# external translator note"));
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}
