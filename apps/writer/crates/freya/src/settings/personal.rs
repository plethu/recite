//! Personal presentation controls; persistence belongs to the shared preference session.
use crate::design::tokens as t;
use crate::editing::Writer;
use freya::prelude::*;
use recite_config::{Keymap, UserConfigEdit as Edit, WriterPaneSide, WriterTheme, WriterView};

pub(super) fn render(
    writer: Writer,
    ids: &[AccessibilityId],
    option_ids: [[AccessibilityId; 2]; 4],
    error: State<String>,
) -> Element {
    let config = writer.preferences.read().config.clone();
    let mut content = rect()
        .width(Size::fill())
        .padding((0., t::SPACE_SM, 0., 0.))
        .spacing(t::SPACE_SM)
        .child(
            label()
                .text(writer.preferences.read().path.clone())
                .font_size(t::TEXT_SMALL),
        );
    for (index, name, labels, selected, edits) in [
        (
            0,
            "Theme",
            ["Light", "Dark"],
            usize::from(config.writer.theme == WriterTheme::Dark),
            [
                Edit::WriterTheme(WriterTheme::Light),
                Edit::WriterTheme(WriterTheme::Dark),
            ],
        ),
        (
            1,
            "Keymap",
            ["Standard", "Vim"],
            usize::from(config.ui.keymap == Keymap::Vim),
            [Edit::Keymap(Keymap::Standard), Edit::Keymap(Keymap::Vim)],
        ),
        (
            2,
            "Preferred view",
            ["Map", "Source"],
            usize::from(config.writer.view == WriterView::Source),
            [
                Edit::WriterView(WriterView::Map),
                Edit::WriterView(WriterView::Source),
            ],
        ),
        (
            3,
            "Script pane",
            ["Left", "Right"],
            usize::from(config.writer.pane_side == WriterPaneSide::Right),
            [
                Edit::WriterPaneSide(WriterPaneSide::Left),
                Edit::WriterPaneSide(WriterPaneSide::Right),
            ],
        ),
    ] {
        content = content.child(crate::design::Options {
            name,
            labels,
            selected,
            ids: option_ids[index],
            change: EventHandler::new(move |index: usize| {
                update(writer, edits[index].clone(), error)
            }),
        });
    }
    for (id, caption, checked, edit) in [
        (
            ids[3],
            "Reduce animation",
            config.writer.reduced_motion,
            Edit::WriterReducedMotion(!config.writer.reduced_motion),
        ),
        (
            ids[4],
            "Zoom around the pointer",
            config.writer.zoom_to_pointer,
            Edit::WriterZoomToPointer(!config.writer.zoom_to_pointer),
        ),
        (
            ids[5],
            "Confirm before closing",
            config.writer.confirm_exit,
            Edit::WriterConfirmExit(!config.writer.confirm_exit),
        ),
    ] {
        content = content.child(crate::design::checkbox(id, caption, checked, move |_| {
            update(writer, edit.clone(), error);
        }));
    }
    content
        .child(
            label()
                .text("Vim: h j k l navigate · i edit · / find a beat · Escape returns to the map")
                .font_size(t::TEXT_SMALL),
        )
        .into_element()
}
fn update(writer: Writer, edit: Edit, mut error: State<String>) {
    if let Edit::WriterView(view) = edit {
        writer.set_view(view);
    } else {
        let mut preferences = writer.preferences;
        if let Err(e) = preferences.write().update(edit) {
            error.set(e);
        }
    }
}
