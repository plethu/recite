#[path = "opening/tests.rs"]
mod opening;
#[path = "structure/tests.rs"]
mod structure;
use super::*;
use crate::{
    Writer,
    keyed::keyed,
    localisation::{Destination, EntryKey, TranslationField},
    tests::support::{self, mounted::Mounted},
    workspaces::tests::ToolDriver,
};
use recite_writer_session::location::Location;
use recite_writer_session::location::Screen;
use std::fs;
use std::sync::Arc;
use tokio::sync::mpsc;
use xilem::{
    WidgetView,
    masonry::{
        core::{
            TextEvent,
            keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers},
        },
        widgets::{Passthrough, TextInput},
    },
};
const FIRST: &str = "11111111111111111111";
const SECOND: &str = "22222222222222222222";
const SOURCE: &str = ":: start default\n> first@11111111111111111111\n  Hello.\n> second@22222222222222222222\n  Another paragraph.\n> missing@33333333333333333333\n  Missing from catalogue.\n-> END\n:: later\n> later@44444444444444444444\n  Another beat.\n-> END\n";
const PO: &str = "msgid \"\"\nmsgstr \"Language: fr\\nPlural-Forms: nplurals=2; plural=(n > 1);\\n\"\n\nmsgctxt \"11111111111111111111&sad\"\nmsgid \"Different wording.\"\nmsgstr \"Variante.\"\n\nmsgctxt \"11111111111111111111\"\nmsgid \"Hello.\"\nmsgid_plural \"Many hellos.\"\nmsgstr[0] \"Bonjour.\"\nmsgstr[1] \"Bonjour à tous.\"\n\nmsgctxt \"22222222222222222222\"\nmsgid \"Another paragraph.\"\nmsgstr \"Autre paragraphe.\"\n";
fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}
fn fixture() -> (tempfile::TempDir, Writer) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(directory.path().join("fr.po"), PO).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer
        .follow_location(&Location {
            screen: Screen::Localise,
            document: "scene.recite".into(),
            beat: Some("start".into()),
            catalogue: Some("fr.po".into()),
            ..Location::default()
        })
        .unwrap();
    (directory, writer)
}
fn field(anchor: &str, arm: usize) -> String {
    format!(
        "Translation · passage {} · form {arm}",
        if anchor == FIRST { 1 } else { 2 }
    )
}
fn row_key(writer: &Writer, index: usize) -> EntryKey {
    writer.localisation.passage.as_ref().unwrap().rows[index]
        .passage()
        .unwrap()
        .translation
        .as_ref()
        .unwrap()
        .key
        .clone()
}
fn undo<S: 'static, V: WidgetView<S, Widget = Passthrough>>(
    mounted: &mut Mounted<S, V>,
    name: &str,
) {
    let id = mounted.find(name);
    let area = mounted
        .harness
        .get_widget_with_id(id)
        .downcast::<TextInput>()
        .map_or(id, |input| input.area_pod().id());
    mounted.harness.focus_on(Some(area));
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
}
#[test]
fn bilingual_fields_keep_independent_native_history_plural_arms_and_frozen_drafts() {
    let (directory, writer) = fixture();
    let first = row_key(&writer, 0);
    let second = row_key(&writer, 1);
    assert_eq!(writer.localisation.passage.as_ref().unwrap().rows.len(), 4);
    assert!(
        writer.localisation.passage.as_ref().unwrap().rows[2]
            .passage()
            .unwrap()
            .translation
            .is_none()
    );
    let mut mounted = Mounted::new(writer, surface);
    let first_widget = mounted.find(&field(FIRST, 1));
    let second_widget = mounted.find(&field(SECOND, 1));
    mounted.replace_text(&field(FIRST, 1), "Premier brouillon.");
    mounted.replace_text(&field(SECOND, 1), "Deuxième brouillon.");
    mounted.state.review_translation(&second, true);
    mounted.state.config.writer.presentation = mounted
        .state
        .config
        .writer
        .presentation
        .with_value(recite_config::WriterPresentationField::UiScale, 200)
        .unwrap();
    mounted.settle();
    let mut checkout = mounted
        .state
        .localisation
        .slot
        .checkout(mounted.state.localisation.mounted_entries())
        .unwrap();
    mounted.settle();
    assert_eq!(mounted.find(&field(FIRST, 1)), first_widget);
    assert_eq!(mounted.find(&field(SECOND, 1)), second_widget);
    assert!(
        mounted.contains_text("Premier brouillon.") && mounted.contains_text("Deuxième brouillon.")
    );
    mounted
        .state
        .localisation
        .slot
        .restore(&mut checkout)
        .unwrap();
    mounted.settle();
    let mut driver = ToolDriver::new(&mut mounted.state);
    mounted
        .state
        .save_focused_translation(&TranslationField::new(second.clone()).0.unwrap());
    assert!(mounted.state.localisation.saving());
    driver.finish(&mut mounted.state);
    mounted.settle();
    let saved = fs::read_to_string(directory.path().join("fr.po")).unwrap();
    assert!(saved.contains("Deuxième brouillon.") && !saved.contains("Premier brouillon."));
    assert!(
        mounted
            .state
            .localisation
            .slot
            .owner()
            .unwrap()
            .catalogue
            .changed(first.id)
    );
    assert!(
        !mounted
            .state
            .localisation
            .slot
            .owner()
            .unwrap()
            .catalogue
            .changed(second.id)
    );
    assert!(
        mounted
            .state
            .localisation
            .slot
            .draft(second.id)
            .unwrap()
            .reviewed
    );
    assert_eq!(
        mounted
            .state
            .localisation
            .slot
            .draft(first.id)
            .unwrap()
            .forms[1],
        "Bonjour à tous."
    );
    assert_eq!(mounted.find(&field(FIRST, 1)), first_widget);
    undo(&mut mounted, &field(FIRST, 1));
    assert_eq!(
        mounted
            .state
            .localisation
            .slot
            .draft(first.id)
            .unwrap()
            .forms[0],
        "Bonjour."
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}
#[test]
fn checked_beat_navigation_retains_catalogue_drafts_and_rejects_hidden_row_save() {
    let (_directory, mut writer) = fixture();
    let first = row_key(&writer, 0);
    writer.update_translation(&first, 0, "Keep across beat navigation.".into());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.inspect("later".into());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(matches!(
        writer.localisation.destination,
        Destination::Passage
    ));
    assert_eq!(writer.localisation.passage.as_ref().unwrap().name, "later");
    writer.save_focused_translation(&TranslationField::new(first.clone()).0.unwrap());
    assert!(!writer.localisation.saving());
    writer.update_translation(&first, 0, "Stale row typing.".into());
    writer.review_translation(&first, true);
    writer.discard_translation(&first);
    writer.inspect("start".into());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(row_key(&writer, 0) == first);
    assert_eq!(
        writer.localisation.slot.draft(first.id).unwrap().forms[0],
        "Keep across beat navigation."
    );
    assert_eq!(writer.current_location().screen, Screen::Localise);
}
#[test]
fn bilingual_page_and_frozen_checkout_cost_stay_bounded_as_the_beat_grows() {
    let mut node_counts = Vec::new();
    for count in [32, 1024] {
        let mut source = ":: start default\n".to_owned();
        let mut po = String::new();
        for index in 0..count {
            source.push_str(&format!("> line@{index:020x}\n  Source {index}.\n"));
            po.push_str(&format!("msgctxt \"{index:020x}\"\nmsgid \"Source {index}.\"\nmsgstr \"Translation {index}.\"\n\n"));
        }
        source.push_str("-> END\n");
        let directory = support::project(&source).unwrap();
        fs::write(directory.path().join("fr.po"), po).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer
            .follow_location(&Location {
                screen: Screen::Localise,
                catalogue: Some("fr.po".into()),
                passage: Some(format!("{:020x}", count - 2)),
                ..Location::default()
            })
            .unwrap();
        let beat = writer.localisation.passage.as_ref().unwrap();
        assert_eq!(beat.total, count + 1);
        assert_eq!(beat.page, (count - 2) / PAGE);
        assert_eq!(beat.rows.len(), PAGE);
        assert_eq!(
            beat.rows[6].passage().unwrap().source.text,
            format!("Source {}.", count - 2)
        );
        let first = row_key(&writer, 0);
        let details = beat.rows[0]
            .passage()
            .unwrap()
            .translation
            .as_ref()
            .unwrap()
            .details
            .clone();
        writer.update_translation(&first, 0, "Typing only changes this draft.".into());
        assert!(Arc::ptr_eq(
            &details,
            &writer.localisation.passage.as_ref().unwrap().rows[0]
                .passage()
                .unwrap()
                .translation
                .as_ref()
                .unwrap()
                .details
        ));
        let selected = writer.localisation.mounted_entries();
        assert_eq!(selected.len(), PAGE);
        let mut checkout = writer.localisation.slot.checkout(selected.clone()).unwrap();
        assert_eq!(
            selected
                .iter()
                .filter(|id| writer.localisation.slot.draft(**id).is_some())
                .count(),
            PAGE
        );
        assert!(
            writer
                .localisation
                .slot
                .draft(
                    checkout
                        .owner
                        .as_ref()
                        .unwrap()
                        .catalogue
                        .document
                        .entries()[0]
                        .id()
                )
                .is_none()
        );
        writer.localisation.slot.restore(&mut checkout).unwrap();
        let mounted = Mounted::new(writer, surface);
        let tree = mounted.harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let (mut nodes, mut fields) = (0, 0);
        while let Some(node) = pending.pop() {
            nodes += 1;
            fields += usize::from(
                node.label()
                    .is_some_and(|label| label.starts_with("Translation · passage ")),
            );
            pending.extend(node.children());
        }
        assert_eq!(fields, PAGE);
        println!("{count} passages: {nodes} native nodes, {fields} translation fields");
        node_counts.push(nodes);
    }
    assert_eq!(node_counts[0], node_counts[1]);
}

#[test]
fn catalogue_replacement_retires_bilingual_rows_before_numeric_ids_can_alias() {
    let (directory, mut writer) = fixture();
    let key = row_key(&writer, 0);
    let replacement = PO.replace("Bonjour.", "Replacement catalogue.");
    let path = directory.path().join("other.po");
    fs::write(&path, replacement).unwrap();
    let mut driver = ToolDriver::new(&mut writer);
    writer.localisation.path = path.display().to_string();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    assert!(writer.localisation.passage.is_none());
    assert!(writer.localisation.slot.identity() != Some(&key.catalogue));
    writer.update_translation(&key, 0, "Stale typing must be ignored.".into());
    assert_eq!(
        writer.localisation.slot.draft(key.id).unwrap().forms[0],
        "Replacement catalogue."
    );
}
