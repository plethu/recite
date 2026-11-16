use super::*;
use crate::app::keyboard::ModalSurface;
use crate::native_host::keyboard::tab::to_tab_stop;
use crate::tests::support::{self, mounted::Mounted};
use xilem::masonry::{
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

