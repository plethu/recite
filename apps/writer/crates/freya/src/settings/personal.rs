//! Personal presentation controls; persistence belongs to the shared preference session.
use crate::editing::Writer;
use freya::prelude::*;
use recite_config::{Keymap, UserConfigEdit as Edit, WriterTheme, WriterView};

pub(super) fn render(writer: Writer, ids: &[AccessibilityId], error: State<String>) -> Element {
    let config = writer.preferences.read().config.clone();
    let mut content = rect().width(Size::fill()).spacing(12.).child(
        label()
            .text(writer.preferences.read().path.clone())
            .font_size(12.),
    );
    for (id, name, value, edit) in [
        (
            ids[0],
            "Theme",
            format!("{:?}", config.writer.theme),
            Edit::WriterTheme(if config.writer.theme == WriterTheme::Light {
                WriterTheme::Dark
            } else {
                WriterTheme::Light
            }),
        ),
        (
            ids[1],
            "Keymap",
            format!("{:?}", config.ui.keymap),
            Edit::Keymap(if config.ui.keymap == Keymap::Standard {
                Keymap::Vim
            } else {
                Keymap::Standard
            }),
        ),
        (
            ids[2],
            "Preferred view",
            format!("{:?}", config.writer.view),
            Edit::WriterView(if config.writer.view == WriterView::Map {
                WriterView::Source
            } else {
                WriterView::Map
            }),
        ),
    ] {
        let name_and_value = format!("{name}: {value}");
        content = content.child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .cross_align(Alignment::Center)
                .width(Size::fill())
                .child(rect().width(Size::flex(1.)).child(label().text(name)))
                .child(super::action(id, name_and_value, move || {
                    update(writer, edit.clone(), error)
                })),
        );
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
        content = content.child(
            rect()
                .width(Size::fill())
                .padding(8.)
                .a11y_id(id)
                .a11y_role(AccessibilityRole::CheckBox)
                .a11y_focusable(true)
                .a11y_alt(caption)
                .a11y_builder(move |node| {
                    node.set_toggled(if checked {
                        accesskit::Toggled::True
                    } else {
                        accesskit::Toggled::False
                    })
                })
                .cursor(CursorIcon::Pointer)
                .border(
                    Border::new()
                        .width(if id.is_focused() { 2. } else { 0. })
                        .fill((120, 130, 115)),
                )
                .on_all_press(move |event: Event<PressEventData>| {
                    event.stop_propagation();
                    id.request_focus();
                    update(writer, edit.clone(), error);
                })
                .child(label().text(format!("{} {caption}", if checked { "☑" } else { "☐" }))),
        );
    }
    content
        .child(
            label()
                .text("Vim: h j k l navigate · i edit · / find a beat · Escape returns to the map")
                .font_size(12.),
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
