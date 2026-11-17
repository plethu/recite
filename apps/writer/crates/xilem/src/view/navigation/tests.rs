use super::*;
use crate::app::keyboard::{RenamesBeat, ReturnFocus};
use crate::native_host::keyboard::focus::slip_anchor;
use crate::workspaces::rename::inline::{Place, Stage, Target};
use crate::workspaces::tests::{ToolDriver, fixture};
use crate::{
    keyed::keyed,
    tests::support::{self, mounted::Mounted},
};
use tokio::sync::mpsc::unbounded_channel;
use xilem::masonry::{
    core::{
        PointerButtonEvent, PointerEvent, PointerId, PointerInfo, PointerState, PointerType,
        WidgetId,
    },
    dpi::PhysicalPosition,
    kurbo::Rect,
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

/// The sidebar beat rows mounted now, by beat.
fn mounted_rows(
    mounted: &Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
) -> Vec<String> {
    let mut rows = Vec::new();
    let mut pending = vec![mounted.harness.root_widget().as_dyn()];
    while let Some(widget) = pending.pop() {
        if widget.ctx().is_stashed() {
            continue;
        }
        if let Some(button) = widget.downcast::<Button>()
            && let Some(target) = button.get_prop::<RenamesBeat>().0.as_ref()
            && target.place == Place::Outline
        {
            rows.push(target.beat.clone());
        }
        pending.extend(widget.children());
    }
    rows
}

/// Where `id` sits in the window.
fn window_bounds(
    mounted: &Mounted<Writer, impl WidgetView<Writer, Widget = Passthrough>>,
    id: WidgetId,
) -> Rect {
    let widget = mounted.harness.get_widget_with_id(id);
    widget
        .ctx()
        .window_transform()
        .transform_rect_bbox(widget.ctx().border_box())
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
    let start = window_bounds(&mounted, row);
    double_click(&mut mounted, row);
    let inline = mounted.state.inline_rename().expect("renaming in the row");
    assert_eq!(
        (inline.beat.as_str(), inline.place, inline.stage),
        ("start", Place::Outline, Stage::Editing)
    );
    // The field and its caption keep to the row's height, so no row below it moves.
    let field = window_bounds(&mounted, mounted.find("New beat id"));
    assert!(
        field.y0 >= start.y0 - 1. && field.y1 <= start.y1 + 1.,
        "{field:?} within the row {start:?}"
    );
    // The row's name is the field; the heading stays a heading.
    mounted.find("New beat id");
    mounted.find("Rename beat");
    mounted.replace_text("New beat id", "opening");
    mounted.state.commit_inline_rename();
    tools.finish(&mut mounted.state);
    mounted.settle();
    // The review floats over the window from under the row (see `app::view` tests), so nothing
    // in the sidebar moves for it.
    assert!(slip_anchor(mounted.harness.render_root()).is_some());
    assert!(!mounted.contains_text("Review all changes"));
    mounted.state.cancel_inline_rename();
    mounted.settle();
    assert!(mounted.state.inline_rename().is_none());
    // The row is back, marked as where focus returns.
    assert!(beat_row(&mounted, "start").1);
}

/// A scene of `count` beats, each leading to the next.
fn long_scene(count: usize) -> String {
    (0..count)
        .map(|index| {
            format!(
                ":: beat_{index}{}\n> @line_{index}\n  Line {index}.\n-> {}\n",
                if index == 0 { " default" } else { "" },
                if index + 1 == count {
                    "END".to_string()
                } else {
                    format!("beat_{}", index + 1)
                }
            )
        })
        .collect()
}

#[test]
fn a_long_scene_scrolls_its_beats_in_a_list_that_keeps_the_selected_beat_in_its_middle() {
    let directory = support::project(&long_scene(40)).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.viewport.width = 1200.;
    let mut mounted = Mounted::new(writer, surface);
    mounted.settle();
    // Navigation runs here, in place of the worker the view starts.
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = unbounded_channel();
    mounted.state.navigation.sender = Some(sender);
    mounted.state.navigation.runtime = Some(runtime.handle().clone());
    // The beats scroll in one named list that is no Tab stop itself; no count stands in for them.
    let list = mounted.find(super::outline::BEATS);
    assert!(
        !mounted
            .harness
            .get_widget_with_id(list)
            .ctx()
            .accepts_focus()
    );
    assert!(!mounted.contains_text("more beats"));
    let view = window_bounds(&mounted, list);
    // Only the rows in view and one past it mount.
    let rows = mounted_rows(&mounted);
    assert!((7..=9).contains(&rows.len()), "{rows:?}");
    assert!(rows.contains(&"beat_0".to_string()));
    assert!(!rows.contains(&"beat_12".to_string()), "{rows:?}");

    // A beat chosen elsewhere, as by Ctrl+Down or Find, scrolls into view, selected and a Tab stop.
    let select = |mounted: &mut Mounted<Writer, _>, jobs: &mut _, beat: &str| {
        mounted.state.inspect(beat.into());
        support::finish_navigation(&mut mounted.state, jobs).unwrap();
        mounted.settle();
    };
    select(&mut mounted, &mut jobs, "beat_30");
    let (row, _) = beat_row(&mounted, "beat_30");
    let bounds = window_bounds(&mounted, row);
    assert!(
        view.y0 - 0.5 <= bounds.y0 && bounds.y1 <= view.y1 + 0.5,
        "{bounds:?} outside {view:?}"
    );
    assert!(
        mounted
            .harness
            .get_widget_with_id(row)
            .ctx()
            .accepts_focus()
    );
    assert!(
        mounted.is_selected("Beat 30"),
        "{:?}",
        mounted_rows(&mounted)
    );
    assert!((7..=10).contains(&mounted_rows(&mounted).len()));

    // The row past the view is built, so Tab goes on to the next beat rather than leaving the list.
    mounted.harness.focus_on(Some(row));
    mounted.harness.press_tab_key(false);
    assert_eq!(
        mounted.harness.focused_widget_id(),
        Some(beat_row(&mounted, "beat_31").0)
    );
    mounted.harness.focus_on(None);

    // A chosen beat comes to the middle, so a row past either edge shows in part while more
    // beats lie beyond it.
    select(&mut mounted, &mut jobs, "beat_28");
    let middle = window_bounds(&mounted, beat_row(&mounted, "beat_28").0);
    assert!(
        (middle.center().y - view.center().y).abs() < 2.,
        "{middle:?} in {view:?}"
    );
    let above = window_bounds(&mounted, beat_row(&mounted, "beat_25").0);
    let below = window_bounds(&mounted, beat_row(&mounted, "beat_31").0);
    assert!(
        above.y0 < view.y0 && above.y1 > view.y0,
        "{above:?} {view:?}"
    );
    assert!(
        below.y0 < view.y1 && below.y1 > view.y1,
        "{below:?} {view:?}"
    );
    assert!(mounted.is_selected("Beat 28"));
}

#[test]
fn the_beat_list_thumb_spans_the_track_from_the_first_beat_to_the_last() {
    let height = 6.5 * 33.;
    let (length, top) = super::outline::thumb_place(40, 0..7, height);
    assert_eq!(top, 2.);
    assert!(length >= 45., "{length}");
    let (_, bottom) = super::outline::thumb_place(40, 33..40, height);
    assert!((bottom + length - (height - 2.)).abs() < 1e-9, "{bottom}");
    // A list that shows every beat fills its track.
    assert_eq!(super::outline::thumb_place(5, 0..5, height).0, height - 4.);
}
