use super::*;
use crate::app::keyboard::{RenamesBeat, ReturnFocus};
use crate::workspaces::rename::inline::{Place, Stage, Target};
use crate::workspaces::tests::{ToolDriver, fixture};
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use xilem::masonry::{
    core::{
        PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType,
        WidgetId,
    },
    dpi::PhysicalPosition,
    widgets::{Button, Passthrough},
};
use xilem::view::PointerButton;

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

/// The sidebar row that renames `beat` in place, and whether focus returns to it.
fn beat_row(
    mounted: &Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
    beat: &str,
) -> (WidgetId, bool) {
    let target = Target {
        beat: beat.into(),
        place: Place::Outline,
    };
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if let Some(button) = widget.downcast::<Button>()
            && button.get_prop::<RenamesBeat>().0.as_ref() == Some(&target)
        {
            return (widget.id(), button.get_prop::<ReturnFocus>().0);
        }
        pending.extend(widget.children());
    }
    panic!("no sidebar row renames {beat}");
}

fn double_click(
    mounted: &mut Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
    id: WidgetId,
) {
    let widget = mounted.harness.get_widget_with_id(id);
    let point = widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
        .center();
    mounted.harness.mouse_move(point);
    for (count, down) in [(1, true), (1, false), (2, true), (2, false)] {
        let event = PointerButtonEvent {
            pointer: PointerInfo {
                pointer_id: Some(PointerId::PRIMARY),
                pointer_type: PointerType::Mouse,
                persistent_device_id: None,
            },
            button: Some(PointerButton::Primary),
            state: PointerState {
                position: PhysicalPosition::new(point.x, point.y),
                count,
                ..PointerState::default()
            },
        };
        mounted.harness.process_pointer_event(if down {
            PointerEvent::Down(event)
        } else {
            PointerEvent::Up(event)
        });
    }
    mounted.settle();
}

#[test]
fn a_double_click_on_a_beat_row_renames_it_in_the_sidebar_and_returns_to_the_row() {
    let directory = fixture().unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.viewport.width = 1200.;
    let mut mounted = Mounted::new(writer, surface);
    let mut tools = ToolDriver::new(&mut mounted.state);
    let (row, _) = beat_row(&mounted, "start");
    double_click(&mut mounted, row);
    let inline = mounted.state.inline_rename().expect("renaming in the row");
    assert_eq!(
        (inline.beat.as_str(), inline.place, inline.stage),
        ("start", Place::Outline, Stage::Editing)
    );
    // The row's name is the field; the heading stays a heading.
    mounted.find("New beat name");
    mounted.find("Rename beat");
    mounted.replace_text("New beat name", "opening");
    mounted.state.commit_inline_rename();
    tools.finish(&mut mounted.state);
    mounted.settle();
    // The slip hangs under the row, with the same actions as the heading's.
    assert!(mounted.contains_text("Renames start to opening in"));
    for action in ["Rename", "Review all changes", "Cancel rename"] {
        mounted.find(action);
    }
    mounted.activate("Cancel rename");
    assert!(mounted.state.inline_rename().is_none());
    // The row is back, marked as where focus returns.
    assert!(beat_row(&mounted, "start").1);
}
