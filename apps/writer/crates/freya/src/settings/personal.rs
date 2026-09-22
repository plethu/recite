//! Personal presentation controls; persistence belongs to the shared preference session.
use crate::design::tokens as t;
use crate::editing::Writer;
use freya::prelude::*;
use recite_config::{Keymap, UserConfigEdit as Edit, WriterPaneSide, WriterTheme, WriterView};

pub(super) fn render(
    writer: Writer,
    ids: &[AccessibilityId],
    option_ids: [[AccessibilityId; 3]; 4],
    size_ids: [AccessibilityId; 6],
    error: State<String>,
    config_id: AccessibilityId,
    mut config_visible: State<bool>,
) -> Element {
    let config = writer.preferences.read().config.clone();
    let mut content = rect()
        .width(Size::fill())
        .padding((0., t::SPACE_SM, 0., 0.))
        .spacing(t::SPACE_SM);
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
            3,
            "Script pane (split view)",
            ["Left", "Right"],
            usize::from(config.writer.pane_side == WriterPaneSide::Right),
            [
                Edit::WriterPaneSide(WriterPaneSide::Left),
                Edit::WriterPaneSide(WriterPaneSide::Right),
            ],
        ),
    ] {
        if index == 0 || index == 1 {
            content = content.child(section(
                writer.dark,
                if index == 0 { "Appearance" } else { "Editing" },
            ));
        }
        content = content.child(crate::design::Options {
            name: name.into(),
            vim: config.ui.keymap == Keymap::Vim,
            labels: labels.map(str::to_owned),
            selected,
            ids: [option_ids[index][0], option_ids[index][1]],
            change: EventHandler::new(move |index: usize| {
                update(writer, edits[index].clone(), error)
            }),
        });
    }
    content = content.child(crate::design::Options {
        name: crate::messages::text(crate::messages::MsgId::WriterWorkspaceWritingView),
        labels: [
            crate::commands::Command::Script.label(writer),
            crate::commands::Command::Map.label(writer),
            crate::commands::Command::Source.label(writer),
        ],
        selected: match config.writer.view {
            WriterView::Script => 0,
            WriterView::Map => 1,
            WriterView::Source => 2,
        },
        ids: option_ids[2],
        vim: config.ui.keymap == Keymap::Vim,
        change: EventHandler::new(move |index| {
            writer.set_view([WriterView::Script, WriterView::Map, WriterView::Source][index])
        }),
    });
    content = content.child(super::typography::controls(writer, size_ids));
    content = content.child(section(writer.dark, "Behaviour"));
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
        .maybe_child((config.ui.keymap == Keymap::Vim).then(|| {
            label()
                .text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiVimHelp,
                ))
                .font_size(t::small())
        }))
        .child(
            crate::design::Button::new()
                .flat()
                .a11y_id(config_id)
                .named(crate::messages::text(
                    crate::messages::MsgId::WriterGuiConfigurationFile,
                ))
                .expanded(*config_visible.read())
                .on_press(move |_| {
                    let next = !*config_visible.peek();
                    config_visible.set(next);
                })
                .child(crate::messages::text(
                    crate::messages::MsgId::WriterGuiConfigurationFile,
                )),
        )
        .maybe_child((*config_visible.read()).then(|| {
            label()
                .text(writer.preferences.read().path.clone())
                .font_size(t::small())
        }))
        .into_element()
}
fn section(dark: bool, title: &'static str) -> Element {
    rect()
        .padding((t::SPACE_SM, 0., 0., 0.))
        .child(
            label()
                .text(title)
                .font_size(t::small())
                .color(crate::palette::muted(dark)),
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
