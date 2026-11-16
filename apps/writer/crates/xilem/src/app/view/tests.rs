use super::*;
use crate::app::keyboard::ModalSurface;
use crate::native_host::keyboard::{focus::slip_anchor, tab::to_tab_stop};
use crate::tests::support::{self, mounted::Mounted};
use crate::workspaces::tests::ToolDriver;
use xilem::masonry::{
    accesskit::Role,
    core::{Widget, WidgetId, WidgetRef},
    widgets::{Passthrough, SizedBox},
};

/// Opens a modal over the window.
type Open = fn(&mut App);

fn page(app: &mut App) -> impl WidgetView<App, Widget = Passthrough> + use<> {
    crate::keyed::keyed((), super::app(app))
}

/// The focusable widgets inside a mounted modal surface.
fn modal_controls(widget: WidgetRef<'_, dyn Widget>, modal: bool, found: &mut Vec<WidgetId>) {
    if widget.ctx().is_stashed() {
        return;
    }
    let modal = modal
        || widget
            .downcast::<SizedBox>()
            .is_some_and(|surface| surface.get_prop::<ModalSurface>().0);
    if modal && widget.ctx().accepts_focus() {
        found.push(widget.id());
    }
    for child in widget.children() {
        modal_controls(child, modal, found);
    }
}

#[test]
fn tab_and_shift_tab_never_reach_the_tab_row_behind_a_modal() {
    let open: [(&str, Open); 2] = [
        ("command picker", |app| app.open_picker(Search::Commands)),
        ("close decision", |app| {
            if let Some(writer) = app.writer.as_mut() {
                writer.change(|model| {
                    model.set_draft("Unsaved line.".into());
                    Ok(())
                });
            }
            app.request_close();
        }),
    ];
    for (modal, open) in open {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut app = App::with_writer(support::open(&directory).unwrap());
        open(&mut app);
        let mut mounted = Mounted::new(app, page);
        mounted.settle();
        let mut inside = Vec::new();
        modal_controls(mounted.harness.root_widget().as_dyn(), false, &mut inside);
        assert!(!inside.is_empty(), "the {modal} is mounted");
        for backwards in [false, true] {
            // Twice round the surface's stops, whichever way Tab goes.
            for _ in 0..2 * inside.len() + 2 {
                mounted.harness.press_tab_key(backwards);
                to_tab_stop(mounted.harness.render_root(), backwards);
                mounted.settle();
                let focused = mounted.harness.focused_widget_id();
                assert!(
                    focused.is_some_and(|id| inside.contains(&id)),
                    "Tab (backwards: {backwards}) left the {modal} for {focused:?}"
                );
            }
        }
    }
}

#[test]
fn the_status_bar_is_inert_behind_the_close_decision() {
    // A missing destination gives the status bar its problem count, which is a button.
    let directory = support::project(":: start default\nHello.\n-> missing\n").unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    if let Some(writer) = app.writer.as_mut() {
        writer.change(|model| {
            model.set_draft("Unsaved line.".into());
            Ok(())
        });
    }
    let problems = |mounted: &Mounted<App, _>| {
        let tree = mounted.harness.access_tree();
        let mut pending = vec![tree.state().root()];
        let mut found = Vec::new();
        while let Some(node) = pending.pop() {
            if node
                .label()
                .is_some_and(|label| label.ends_with("problem") || label.ends_with("problems"))
                && node.role() == Role::Button
            {
                found.push(node.is_disabled());
            }
            pending.extend(node.children());
        }
        found
    };
    let mut mounted = Mounted::new(app, page);
    mounted.settle();
    assert_eq!(
        problems(&mounted),
        vec![false],
        "the count is live while writing"
    );
    mounted.state.request_close();
    mounted.settle();
    assert_eq!(
        problems(&mounted),
        vec![true],
        "the count is inert behind the decision"
    );
}

#[test]
fn a_reviewed_rename_hangs_its_slip_under_the_heading_and_moves_nothing() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let app = App::with_writer(support::open(&directory).unwrap());
    let mut mounted = Mounted::new(app, page);
    mounted.settle();
    let writer = mounted.state.writer.as_mut().unwrap();
    let mut tools = ToolDriver::new(writer);
    writer.rename_beat();
    writer.edit_inline_rename("renamed".into());
    writer.commit_inline_rename();
    tools.finish(writer);
    mounted.settle();
    // The native host measures the heading and hangs the slip from it.
    let heading = slip_anchor(mounted.harness.render_root()).expect("the heading is the anchor");
    let writer = mounted.state.writer.as_mut().unwrap();
    writer.place_inline_rename(heading.0, heading.1);
    mounted.settle();
    assert_eq!(slip_anchor(mounted.harness.render_root()), Some(heading));
    let rename = mounted.find("Rename");
    let widget = mounted.harness.get_widget_with_id(rename);
    let top = (widget.ctx().window_transform() * widget.ctx().border_box().origin()).y;
    assert!(
        top > f64::from(heading.1),
        "the slip hangs below the heading"
    );
    for action in ["Review all changes", "Cancel rename"] {
        mounted.find(action);
    }
    mounted.activate("Cancel rename");
    assert!(
        mounted
            .state
            .writer
            .as_ref()
            .unwrap()
            .inline_rename()
            .is_none()
    );
}
