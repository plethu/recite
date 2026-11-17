use super::*;
use crate::native_host::keyboard::focus::slip_anchor;
use crate::workspaces::tests::{ToolDriver, fixture};
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use xilem::masonry::widgets::{Passthrough, TextInput};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

/// The window y of the top of the control named `name`.
fn top<V: WidgetView<Writer, Widget = Passthrough>>(
    mounted: &Mounted<Writer, V>,
    name: &str,
) -> f64 {
    let widget = mounted.harness.get_widget_with_id(mounted.find(name));
    let ctx = widget.ctx();
    (ctx.window_transform() * ctx.border_box().origin()).y
}

fn relay_hub() -> Writer {
    let mut writer = Writer::from_session(crate::session::Session::examples().unwrap(), None);
    writer
        .follow_location(&recite_writer_session::location::Location {
            document: "relay_hub.recite".into(),
            ..Default::default()
        })
        .unwrap();
    writer.viewport.width = 1200.;
    writer
}

#[test]
fn the_galley_sets_the_whole_scene_with_only_the_selected_beat_editable() {
    let mounted = Mounted::new(relay_hub(), surface);
    let total = mounted.state.reading.script().unwrap().len();
    assert!(total > 1);
    // Every other beat is a named heading button that says what pressing it does.
    let second = mounted.state.reading.script().unwrap()[1].id.clone();
    mounted.find(&format!(
        "Edit beat {}, 2 of {total}",
        display_name(&second)
    ));
    // Read-only beats expose their dialogue as text in reading order, not only through the button.
    let rows = mounted.state.reading.beat_rows(1).unwrap();
    let line = rows
        .iter()
        .find_map(|row| match &row.content {
            Content::Passage { passage, .. } => Some(passage.text.clone()),
            _ => None,
        })
        .unwrap();
    assert!(mounted.contains_text(&line), "{line}");
    // Editable fields belong to the selected beat alone.
    let fields = |mounted: &Mounted<Writer, _>| {
        let mut pending = vec![mounted.harness.root_widget().as_dyn()];
        let mut count = 0;
        while let Some(widget) = pending.pop() {
            if !widget.ctx().is_stashed() && widget.downcast::<TextInput>().is_some() {
                count += 1;
            }
            pending.extend(widget.children());
        }
        count
    };
    let selected_rows = mounted.state.reading.rows.len();
    assert!(
        fields(&mounted) <= selected_rows + 2,
        "only one beat mounts fields"
    );
    // The selected beat keeps its own actions beside its heading.
    for action in ["Rename beat", "Pin reference", "Copy link"] {
        mounted.find(action);
    }
}

#[test]
fn rename_beat_turns_the_heading_into_a_field_and_its_review_into_a_slip() {
    let directory = fixture().unwrap();
    let writer = support::open(&directory).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    let mut tools = ToolDriver::new(&mut mounted.state);
    // Measured as the window lays it out: the field's face comes from the theme.
    let properties = crate::presentation::Theme::new(&mounted.state.config.writer).properties;
    mounted.harness.set_default_properties(properties);
    mounted.settle();
    let row = top(&mounted, "Copy link");
    mounted.activate("Rename beat");
    let beat = mounted.state.inline_rename().unwrap().beat.clone();
    // The field is captioned with what it edits: the beat's id.
    assert!(mounted.contains_text("Beat id"));
    // It takes the heading's line, so the heading row keeps its height and nothing moves.
    assert_eq!(top(&mounted, "Copy link"), row);
    mounted.replace_text("New beat id", "renamed");
    mounted.state.commit_inline_rename();
    tools.finish(&mut mounted.state);
    mounted.settle();
    // The field gives way to the heading, which the review's slip hangs from over the window
    // (see `app::view` tests): nothing in the galley moves for it.
    assert!(!mounted.contains_text("New beat id"));
    assert!(slip_anchor(mounted.harness.render_root()).is_some());
    assert!(!mounted.contains_text("Review all changes"));
    assert_eq!(mounted.state.inline_rename().unwrap().beat, beat);
    mounted.state.cancel_inline_rename();
    mounted.settle();
    assert!(mounted.state.inline_rename().is_none());
    mounted.find("Rename beat");
}
