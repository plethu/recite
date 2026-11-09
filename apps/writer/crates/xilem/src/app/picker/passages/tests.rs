#![allow(unused_imports)]
use super::*;
use crate::Phase;
use crate::Writer;
use crate::app::picker;
use crate::session::Session;
use recite_config::WriterCommand;
use recite_writer_session::location::Location;
use recite_writer_session::startup::{LaunchOptions, Startup};
use std::path::PathBuf;
use std::sync::Arc;

fn long_beat(passages: usize) -> String {
    let mut source = String::from(":: start default\n");
    for n in 1..=passages {
        source.push_str(&format!("> @line_{n}\n  Courier {n} waits in the café.\n"));
    }
    source.push_str("-> END\n");
    source
}

#[test]
fn saved_text_and_current_beat_lookup_open_the_matching_off_page_passage() {
    for mode in [Search::ProjectText, Search::Passages] {
        let directory = support::project(&long_beat(40)).unwrap();
        let mut app = App::with_writer(support::open(&directory).unwrap());
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let (sender, mut jobs) = async_mpsc::unbounded_channel();
        let writer = app.writer.as_mut().unwrap();
        writer.navigation.sender = Some(sender);
        writer.navigation.runtime = Some(runtime.handle().clone());
        writer.change(|model| {
            model.set_draft("Keep this outgoing draft.".into());
            Ok(())
        });
        app.open_picker(mode);
        app.filter_picker("Courier 39".into());
        assert_eq!(app.picker.as_ref().unwrap().matches.len(), 1);
        app.activate_picker(0);
        assert!(app.picker.is_none());
        let writer = app.writer.as_mut().unwrap();
        support::finish_navigation(writer, &mut jobs).unwrap();
        assert_eq!(
            writer.session.model().view(),
            &recite_writer_model::View::Passage(format!("line_{}", 39))
        );
        assert_eq!(writer.reading.window.start(), 32);
        assert_eq!(
            writer.reading.selected_passage().unwrap().text,
            "Courier 39 waits in the café."
        );
        assert!(
            writer
                .session
                .model()
                .document()
                .source()
                .contains("Keep this outgoing draft.")
        );
        writer.reading.window = Default::default();
        app.open_picker(mode);
        app.filter_picker("Courier 39".into());
        app.activate_picker(0);
        let writer = app.writer.as_mut().unwrap();
        support::finish_navigation(writer, &mut jobs).unwrap();
        assert_eq!(
            writer.reading.window.start(),
            32,
            "reopening the selected passage reveals its page again"
        );
    }
}

#[test]
fn accepted_navigation_reveals_the_destination_rail_without_opening_other_documents() {
    let directory = support::project(&long_beat(40)).unwrap();
    for n in 0..9 {
        let source = (0..20)
            .map(|beat| format!(":: beat_{beat}\n-> END\n"))
            .collect::<String>();
        std::fs::write(directory.path().join(format!("other_{n}.recite")), source).unwrap();
    }
    let mut app = App::with_writer(support::open(&directory).unwrap());
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    let writer = app.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.navigate(crate::navigation::Target::Location(Location {
        document: "other_8.recite".into(),
        beat: Some("beat_18".into()),
        ..Location::default()
    }));
    support::finish_navigation(writer, &mut jobs).unwrap();
    assert_eq!(writer.beat_page, 2);
    assert_eq!(writer.file_page, 2);
    assert_eq!(writer.session.project().unwrap().sessions().count(), 2);
    writer.file_page = 0;
    writer.beat_page = 0;
    writer.configure(writer.config.clone());
    assert_eq!(
        (writer.file_page, writer.beat_page),
        (0, 0),
        "ordinary presentation must not snap a browsed rail page back"
    );
}

#[test]
fn long_beat_lookup_shares_prose_and_keeps_native_result_mounting_constant() {
    let mut mounted_counts = Vec::new();
    for passages in [40, 1040] {
        let directory = support::project(&long_beat(passages)).unwrap();
        let mut app = App::with_writer(support::open(&directory).unwrap());
        app.open_picker(Search::Passages);
        app.filter_picker("Courier".into());
        assert_eq!(app.picker.as_ref().unwrap().matches.len(), passages);
        let mut ctx = ViewCtx::new(
            Arc::new(NoProxy),
            Arc::new(tokio::runtime::Runtime::new().unwrap()),
        );
        let view = view::picker(&app);
        let (pod, _) = view.build(&mut ctx, &mut app);
        let harness = TestHarness::create_with_size(
            app.theme.initial_properties(),
            NewWidget::new(SizedBox::new(pod.new_widget)),
            (1280, 800),
        );
        let tree = harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut nodes = 0;
        let mut buttons = 0;
        while let Some(node) = pending.pop() {
            nodes += 1;
            buttons += usize::from(node.role() == accesskit::Role::Button);
            pending.extend(node.children());
        }
        assert!(buttons <= 9);
        mounted_counts.push(nodes);
        let preview = app
            .picker
            .as_ref()
            .unwrap()
            .previews
            .get(0)
            .unwrap()
            .as_ptr();
        app.step_picker(true);
        app.step_picker(true);
        assert_eq!(
            app.picker
                .as_ref()
                .unwrap()
                .previews
                .get(0)
                .unwrap()
                .as_ptr(),
            preview
        );
        app.next_picker_page();
        let picker = app.picker.as_ref().unwrap();
        assert!(picker.previews.get(PAGE).unwrap().contains("Courier 6"));
        app.filter_picker(format!("Courier {passages}"));
        let picker = app.picker.as_ref().unwrap();
        assert_eq!(picker.matches.len(), 1);
        assert!(
            picker
                .previews
                .get(0)
                .unwrap()
                .contains(&format!("Courier {passages}"))
        );
        assert!(picker.previews.get(1).is_none());
        let Item::Passage { passage, .. } = &picker.entries[picker.matches[0]].item else {
            panic!("accepted passage result");
        };
        let original = app
            .writer
            .as_ref()
            .unwrap()
            .reading
            .rows
            .iter()
            .find_map(|row| match &row.content {
                crate::reading::Content::Passage {
                    passage: original, ..
                } if original.id == passage.id => Some(original),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            passage.text,
            format!("Courier {passages} waits in the café.")
        );
        assert_eq!(passage.text.as_ptr(), original.text.as_ptr());
    }
    assert_eq!(mounted_counts[0], mounted_counts[1]);
}

#[test]
fn partial_word_lookup_previews_the_late_matching_text() {
    let text = format!(
        "{}İstanbul's Courier’s packet waits.",
        "A quiet hallway. ".repeat(500)
    );
    let source = format!(":: start default\n> @ffee6p8\n  {text}\n-> END\n");
    let directory = support::project(&source).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.open_picker(Search::Passages);
    for query in ["cour", "cour pack", "courier’s", "İST"] {
        app.filter_picker(query.into());
        let picker = app.picker.as_ref().unwrap();
        assert_eq!(picker.matches.len(), 1);
        let excerpt = picker.previews.get(0).unwrap();
        assert!(
            excerpt.contains("İstanbul's Courier’s packet"),
            "{query}: {excerpt}"
        );
        assert!(excerpt.starts_with("… "));
    }
    app.open_picker(Search::ProjectText);
    app.filter_picker("cour".into());
    assert_eq!(
        app.picker.as_ref().unwrap().total,
        0,
        "saved search keeps complete-word matching"
    );
}

#[test]
fn speaker_only_saved_search_keeps_its_matching_context_visible() {
    let source = ":: start default\n> speaker=mara @ffee6p8\n  The station is quiet.\n-> END\n";
    let directory = support::project(source).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.open_picker(Search::ProjectText);
    app.filter_picker("mara".into());
    let picker = app.picker.as_ref().unwrap();
    assert_eq!(picker.total, 1);
    assert!(picker.entries[0].title.contains("mara"));
    assert_eq!(picker.entries[0].detail, "The station is quiet.");
}
