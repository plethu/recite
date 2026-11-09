use crate::localisation::{
    tests::{fixture, form, key},
    work,
};
use std::fs;

#[test]
fn catalogue_scope_survives_checkout_and_rejects_reopened_same_path_and_saved_baselines() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Pending translation.".into());
    let request = writer.localisation.watch_request().unwrap();
    let scope = writer.localisation.watch_scope().unwrap();
    let mut checkout = writer.localisation.slot.checkout(Some(key.id)).unwrap();
    assert!(writer.localisation.watch_scope().as_ref() == Some(&scope));
    assert!(writer.localisation.watch_request().is_none());
    let path = directory.path().join("fr.po");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(&path, original.replace("Bonjour.", "External translation.")).unwrap();
    let observation = request.observe();
    assert!(observation.changed());
    assert!(!observation.is_current(&writer.localisation));
    writer.localisation.slot.restore(&mut checkout).unwrap();
    assert!(observation.is_current(&writer.localisation));
    assert_eq!(form(&writer, &key), "Pending translation.");
    fs::write(&path, &original).unwrap();
    let before_save = writer.localisation.watch_request().unwrap();
    writer.save_translation_drafts();
    driver.finish(&mut writer);
    assert!(!before_save.observe().is_current(&writer.localisation));
    let before_reopen = writer.localisation.watch_request().unwrap();
    writer.catalogue_action(work::Action::Reload);
    driver.finish(&mut writer);
    assert!(!before_reopen.observe().is_current(&writer.localisation));
    assert!(writer.localisation.watch_scope().as_ref() != Some(&scope));
}
