use super::*;
use crate::{
    tests::support::{self, mounted::Mounted},
    workspaces::tests::ToolDriver,
};
use recite_core::po::PoDocument;
use std::fs;
use xilem::masonry::widgets::Passthrough;

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), crate::view::writer(writer))
}

#[test]
fn plural_pages_keep_every_form_and_save_the_reviewed_entry_without_changing_source() {
    let text = "A long source passage remains inspectable. ".repeat(60);
    let source = format!(
        ":: start default\n> line@11111111111111111111\n  {text}\n  | Multiple tickets.\n-> END\n"
    );
    let directory = support::project(&source).unwrap();
    let path = directory.path().join("fr.po");
    let rule = format!("nplurals=6; plural=(n{}%6);", " ".repeat(2200));
    let mut po = format!(
        "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: {rule}\\n\"\n\n#, fuzzy\nmsgctxt \"11111111111111111111\"\nmsgid \"{text}\"\nmsgid_plural \"Multiple tickets.\"\n"
    );
    for index in 0..6 {
        po.push_str(&format!("msgstr[{index}] \"form {index}\"\n"));
    }
    fs::write(&path, &po).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut driver = ToolDriver::new(&mut writer);
    writer.open_localisation();
    writer.localisation.path = path.display().to_string();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    let key = EntryKey {
        catalogue: writer.localisation.slot.identity().unwrap().clone(),
        id: writer.localisation.queue.rows[0].id,
    };
    writer.inspect_translation(&key);
    driver.finish(&mut writer);
    assert!(
        writer
            .localisation
            .entry
            .as_ref()
            .unwrap()
            .details
            .plural
            .valid()
    );
    assert!(writer.localisation.entry.as_ref().unwrap().rule.pages.len() > 1);
    let mut mounted = Mounted::new(writer, surface);
    let mut driver = ToolDriver::new(&mut mounted.state);
    mounted.replace_text("Translation form 1", "Un billet.");
    let field = mounted.find("Translation form 1");
    mounted.type_text("Translation form 1", "x");
    let mut config = mounted.state.config.clone();
    config.writer.presentation = config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    mounted.state.configure(config);
    mounted.settle();
    assert_eq!(
        mounted.find("Translation form 1"),
        field,
        "responsive entry keeps the native editor"
    );
    use xilem::masonry::core::{
        TextEvent,
        keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers},
    };
    mounted
        .harness
        .process_text_event(TextEvent::Keyboard(KeyboardEvent {
            state: KeyState::Down,
            key: Key::Character("z".into()),
            code: Code::KeyZ,
            modifiers: if cfg!(target_os = "macos") {
                Modifiers::META
            } else {
                Modifiers::CONTROL
            },
            ..KeyboardEvent::default()
        }));
    mounted.settle();
    assert_eq!(
        mounted.state.localisation.slot.draft(key.id).unwrap().forms[0],
        "Un billet."
    );
    mounted.activate("More plural forms");
    assert_eq!(
        mounted.state.localisation.entry.as_ref().unwrap().form_page,
        1
    );
    mounted.replace_text("Translation form 5", "Cinq billets.");
    mounted.activate("Previous plural forms");
    assert_eq!(
        mounted.state.localisation.slot.draft(key.id).unwrap().forms[0],
        "Un billet."
    );
    mounted.activate("Plural rule");
    mounted.activate("More plural rule");
    assert_eq!(
        mounted.state.localisation.entry.as_ref().unwrap().rule.page,
        1
    );
    mounted.activate("Previous plural rule part");
    mounted.activate("Hide plural rule");
    mounted.activate("More translation source");
    assert_eq!(
        mounted
            .state
            .localisation
            .entry
            .as_ref()
            .unwrap()
            .source_page,
        1
    );
    mounted.activate("Previous translation source part");
    mounted.activate("Reviewed translation");
    assert_eq!(fs::read_to_string(&path).unwrap(), po);
    mounted.activate("Save translation");
    driver.finish(&mut mounted.state);
    mounted.settle();
    let saved = PoDocument::parse(fs::read_to_string(&path).unwrap()).unwrap();
    let entry = saved
        .entries()
        .iter()
        .find(|entry| entry.context() == Some("11111111111111111111"))
        .unwrap();
    assert_eq!(entry.plural_translations()[0].text(), "Un billet.");
    assert_eq!(entry.plural_translations()[4].text(), "Cinq billets.");
    assert_eq!(entry.plural_translations()[5].text(), "form 5");
    assert!(!entry.flags().iter().any(|flag| flag == "fuzzy"));
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        source
    );
    assert!(!mounted.state.localisation.saving());
}
