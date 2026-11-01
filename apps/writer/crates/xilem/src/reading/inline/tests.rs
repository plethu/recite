use crate::{
    Writer,
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
    view,
};
use masonry::{
    accesskit::TextSelection,
    core::{
        TextEvent, WidgetId,
        keyboard::{Code, Key, KeyState, KeyboardEvent, Modifiers},
    },
    widgets::{Passthrough, TextArea},
};
use recite_writer_model::{View, Workbench};
use xilem::WidgetView;

fn page(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), view::writer(writer))
}

fn selection(
    mounted: &Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
    id: WidgetId,
) -> Option<TextSelection> {
    let tree = mounted.harness.access_tree();
    let mut pending = vec![tree.state().root()];
    while let Some(node) = pending.pop() {
        if node.locate().0.0 == id.to_raw() {
            return node.raw_text_selection().cloned();
        }
        pending.extend(node.children());
    }
    None
}

#[test]
fn initial_edit_keeps_the_whole_beat_and_rejected_navigation_keeps_its_draft() {
    let directory = support::project(
        ":: start default\n> @ffee6p8\n  Hello.\n? @rthh7ab\n  What happened?\n  -> END\n",
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    assert_eq!(writer.reading.rows.len(), 2);
    assert_eq!(writer.reading.selected_row, Some(0));
    assert_eq!(writer.reading.replies, 1);
    let rows = writer.reading.rows.clone();
    let selected = writer.session.model().view().clone();
    writer.change(|model| {
        model.set_draft("-> syntax".into());
        Ok(())
    });
    assert!(std::sync::Arc::ptr_eq(&rows, &writer.reading.rows));
    let labels = support::labels(&mut writer).unwrap();
    assert!(labels.iter().any(|label| label == "What happened?"));
    support::click(&mut writer, &["Edit passage 2"]).unwrap();
    assert_eq!(writer.session.model().view(), &selected);
    assert_eq!(writer.session.model().draft(), "-> syntax");
    assert_eq!(writer.reading.rows.len(), 2);
}

#[test]
fn conditional_headings_do_not_consume_accessible_passage_numbers() {
    let source = ":: start default\n:if ready()\n  > @ffee6p8\n    First.\n:else\n  > @rthh7ab\n    Second.\n-> END\n";
    let directory = support::project(source).unwrap();
    let mut writer = support::open(&directory).unwrap();
    support::click(&mut writer, &["Project", "Read beat"]).unwrap();
    let labels = support::labels(&mut writer).unwrap();
    let fields = labels
        .iter()
        .filter(|label| label.starts_with("Edit passage "))
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        fields,
        ["Edit passage 1", "Edit passage 2"].into_iter().collect()
    );
    support::click(&mut writer, &["Edit passage 2"]).unwrap();
    assert_eq!(
        writer.session.model().view(),
        &View::Passage("rthh7ab".into())
    );
    assert_eq!(writer.session.model().draft(), "Second.");
    assert_eq!(writer.session.model().document().source(), source);
}

#[test]
fn preceding_siblings_can_change_without_replacing_the_active_native_field_or_undo() {
    let original = ":: start default\n> @ffee6p8\n  Original.\n> @rthh7ab\n  Afterwards.\n-> END\n";
    let inserted = original.replace(
        ":: start default\n",
        ":: start default\n> @bqyvpqq\n  Before.\n",
    );
    let directory = support::project(original).unwrap();
    let mut mounted = Mounted::new(support::open(&directory).unwrap(), page);
    mounted.replace_text("Manuscript", "Edited.");
    let field = mounted.find("Manuscript");
    let area = field;
    mounted.harness.edit_widget_with_id(area, |mut widget| {
        TextArea::<true>::select_byte_range(&mut widget.downcast(), 1, 5)
    });
    mounted.settle();
    let selected = selection(&mounted, area);
    assert!(selected.is_some());
    for source in [&inserted, original] {
        mounted.state.change(|model| {
            let draft = model.draft().to_owned();
            let mut replacement = Workbench::open(model.document().key().as_str(), source)?;
            replacement.select(View::Passage("ffee6p8".into()))?;
            replacement.set_draft(draft);
            *model = replacement;
            Ok(())
        });
        mounted.settle();
        assert_eq!(mounted.find("Manuscript"), field);
        assert_eq!(
            mounted.harness.focused_widget().map(|widget| widget.id()),
            Some(area)
        );
        assert_eq!(selection(&mounted, area), selected);
    }
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
    assert_eq!(mounted.state.session.model().draft(), "Original.");
    assert_eq!(mounted.find("Manuscript"), field);
}
