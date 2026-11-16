use crate::localisation::{
    tests::{PO, SOURCE},
    work::Action,
};
use crate::{test_support as support, workspaces::tests::ToolDriver};
use std::fs;

#[test]
fn the_caption_names_the_catalogue_language_and_falls_back_to_the_file_alone() {
    let directory = support::project(SOURCE).unwrap();
    let path = directory.path().join("fr.po");
    fs::write(&path, PO).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut driver = ToolDriver::new(&mut writer);
    writer.open_localisation();
    writer.localisation.path = path.display().to_string();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    assert_eq!(writer.localisation.language_name(), None);
    assert_eq!(writer.localisation.catalogue_caption(), "fr.po");

    writer.catalogue_action(Action::Close);
    driver.finish(&mut writer);
    fs::write(
        &path,
        format!("msgid \"\"\nmsgstr \"Language: fr\\n\"\n\n{PO}"),
    )
    .unwrap();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    assert_eq!(
        writer.localisation.language_name().as_deref(),
        Some("French")
    );
    assert_eq!(writer.localisation.catalogue_caption(), "French · fr.po");
}
