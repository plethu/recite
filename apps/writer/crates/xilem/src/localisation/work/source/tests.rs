use super::*;
use crate::localisation::tests::{fixture, form, key};
use std::fs;

#[test]
fn changing_sources_after_review_requires_a_new_review_without_publishing_po() {
    let (directory, mut writer, mut driver) = fixture();
    writer.review_catalogue_sources();
    driver.finish(&mut writer);
    let before = fs::read(directory.path().join("fr.po")).unwrap();
    writer
        .session
        .edit(|model| {
            model.set_draft("Newer source.".into());
            model.apply()
        })
        .unwrap();
    writer.apply_catalogue_sources();
    driver.finish(&mut writer);
    assert_eq!(fs::read(directory.path().join("fr.po")).unwrap(), before);
    assert!(!writer.session.read_only() && !writer.localisation.working());
    assert!(writer.localisation.refresh.is_some());
    assert!(writer.status.contains("changed"), "{}", writer.status);
}

#[test]
fn failed_dispatch_returns_both_owners_with_the_unresolved_manuscript_field() {
    let (_directory, mut writer, _driver) = fixture();
    let key = key(&writer);
    writer
        .session
        .edit(|model| {
            model.set_draft("Pending field.".into());
            Ok(())
        })
        .unwrap();
    drop(_driver);
    writer.review_catalogue_sources();
    assert!(!writer.session.read_only() && !writer.localisation.working());
    assert!(writer.localisation.slot.identity() == Some(&key.catalogue));
    assert_eq!(writer.session.model().draft(), "Pending field.");
    assert!(
        writer.status.contains("worker stopped"),
        "{}",
        writer.status
    );
}

#[test]
fn unwind_returns_and_quarantines_both_real_owners() {
    let (_directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    let project = writer.session.checkout().unwrap();
    let catalogue = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    let mut completed = Work {
        project,
        catalogue,
        action: Action::Review,
        query: Query::default(),
    }
    .execute_with(|_, _, _, _| panic!("injected source operation failure"));
    writer.accept_catalogue_source(&mut completed).unwrap();
    assert!(!writer.session.checked_out() && !writer.localisation.working());
    assert!(writer.session.recovery_reason() == Some(RecoveryReason::ReopenRequired));
    assert!(writer.localisation.slot.read_only());
    assert!(writer.localisation.slot.identity() == Some(&key.catalogue));
    writer.request_close();
    writer.close_with_recovery();
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
}

#[test]
fn creation_returns_published_owner_even_if_cancellation_arrives_after_the_write() {
    let (directory, mut writer, mut driver) = fixture();
    let source = fs::read(directory.path().join("scene.recite")).unwrap();
    writer.localisation.creation.selected = "fr-CA".into();
    writer.create_catalogue();
    assert!(writer.session.checked_out() && writer.localisation.working());
    driver.complete_after(&mut writer, Writer::cancel_catalogue_action);
    let path = directory.path().join("locale/fr-CA.po");
    assert_eq!(writer.localisation.slot.path(), Some(path.as_path()));
    assert!(!writer.session.read_only() && !writer.localisation.slot.read_only());
    assert_eq!(
        fs::read(directory.path().join("scene.recite")).unwrap(),
        source
    );
    let document = &writer.localisation.slot.owner().unwrap().catalogue.document;
    assert!(
        document
            .entries()
            .iter()
            .filter(|entry| !entry.is_header())
            .all(|entry| entry.translation() == Some(""))
    );
    assert!(
        document
            .headers()
            .iter()
            .any(|header| header.key() == "Language" && header.value() == "fr-CA")
    );
    assert_eq!(fs::read_to_string(&path).unwrap(), document.source());
}

#[test]
fn source_review_is_observational_and_apply_writes_only_catalogue() {
    let (directory, mut writer, mut driver) = fixture();
    let original_source = fs::read_to_string(directory.path().join("scene.recite")).unwrap();
    writer
        .session
        .edit(|model| {
            model.set_draft("Changed source.".into());
            model.apply()
        })
        .unwrap();
    writer.review_catalogue_sources();
    assert!(writer.session.checked_out() && writer.localisation.working());
    driver.finish(&mut writer);
    assert!(
        !writer.session.read_only() && !writer.localisation.working(),
        "{}",
        writer.status
    );
    let refresh = writer.localisation.refresh.as_ref().unwrap();
    assert_eq!(refresh.preview.changes.len(), 1);
    assert!(
        refresh.pages[0]
            .detail
            .as_ref()
            .unwrap()
            .contains("Bonjour.")
    );
    assert!(
        !fs::read_to_string(directory.path().join("fr.po"))
            .unwrap()
            .contains("Changed source.")
    );
    let old = key(&writer);
    writer.apply_catalogue_sources();
    driver.finish(&mut writer);
    assert!(
        fs::read_to_string(directory.path().join("fr.po"))
            .unwrap()
            .contains("Changed source.")
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        original_source
    );
    assert!(writer.localisation.slot.identity() != Some(&old.catalogue));
    writer.update_translation(&old, 0, "Stale input".into());
    let id = writer.localisation.queue.rows[0].id;
    assert_eq!(
        writer.localisation.slot.draft(id).unwrap().forms,
        ["Bonjour."]
    );
}

#[test]
fn cancelled_source_action_returns_both_owners_and_keeps_the_native_draft() {
    let (_directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Translation draft".into());
    writer.review_catalogue_sources();
    writer.cancel_catalogue_action();
    driver.finish(&mut writer);
    assert!(!writer.session.read_only() && !writer.localisation.working());
    assert_eq!(form(&writer, &key), "Translation draft");
    assert!(writer.localisation.refresh.is_none());
}
