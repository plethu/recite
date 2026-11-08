use super::*;

#[test]
fn replacing_a_project_accepts_clean_catalogues_but_preserves_drafts_and_exclusive_owners() {
    assert!(Localisation::default().can_leave());
    for state in ["clean", "draft", "working", "saving", "quarantined"] {
        let (_directory, mut writer, mut driver) = fixture();
        match state {
            "draft" => {
                let key = key(&writer);
                writer.update_translation(&key, 0, "Keep this translation.".into());
            }
            "working" => writer.search_catalogue(),
            "saving" => {
                let _request = writer
                    .localisation
                    .slot
                    .owner_mut()
                    .unwrap()
                    .catalogue
                    .begin_save(None)
                    .unwrap();
            }
            "quarantined" => writer.localisation.slot.owner_mut().unwrap().quarantine(),
            _ => (),
        }
        assert_eq!(writer.localisation.can_leave(), state == "clean", "{state}");
        if state == "working" {
            writer.cancel_catalogue_action();
            driver.finish(&mut writer);
            assert!(writer.localisation.can_leave());
        }
    }
}
