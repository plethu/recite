use super::*;
use crate::{keyed::keyed, tests::support::mounted::Mounted};
use xilem::masonry::widgets::{Passthrough, TextInput};

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
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
