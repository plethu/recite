//! Shared scene identity and explicit writing views.
use crate::commands::CommandExt;
mod menu;
use crate::{
    commands::Command,
    design::{Segments, tokens as t},
    editing::{Pane, Writer},
    messages::{MsgId, text},
    palette,
};
use freya::prelude::*;
use recite_config::WriterView;

fn focus_mode(writer: Writer) -> bool {
    *writer.layout.focus.read()
}

pub(super) fn toolbar(writer: Writer, actions: Option<Element>) -> Element {
    let activity = crate::localisation::switch(writer);
    let localising = writer.localisation.read().active;
    let options = menu::render(writer, !localising);
    let ids = [use_a11y(), use_a11y(), use_a11y()];
    let state = writer.buffers.model.read();
    let Ok(session) = state.as_ref() else {
        return rect().into_element();
    };
    let compact = *writer.layout.available.read() < 1100. * t::ui_scale();
    let rail = if writer.layout.navigation_expanded() {
        *writer.layout.drawer_width.read()
    } else {
        t::collapsed_drawer_width()
    };
    // Wide content gets one toolbar row: crumbs, views, actions, then the workspace controls.
    // The window width is known on the first frame, so the row count never flips after layout.
    // Layout sizes are device pixels; the threshold and rail width are logical.
    let platform = Platform::get();
    let window = platform.root_size.read().width / *platform.scale_factor.read() as f32;
    let one_row =
        !focus_mode(writer) && t::ui_scale() <= 1.25 && window - rail >= 1000. * t::ui_scale();
    let focus = *writer.layout.focus.read();
    let view = *writer.layout.view.read();
    let scene = palette::display_name(session.document().key().as_str());
    let tools = !writer.localisation.read().active
        && matches!(*writer.pane.read(), Pane::Map | Pane::Script);
    let title = label()
        .text(scene)
        .font_size(t::heading())
        .max_lines(1)
        .text_overflow(TextOverflow::Ellipsis);
    let mut header = rect()
        .horizontal()
        .width(Size::fill())
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .maybe_child((!focus).then(|| crate::navigation::buttons(writer)))
        .child(title.clone().width(Size::flex(1.)))
        .child(Command::Commands.button(writer))
        .maybe_child((!focus && t::ui_scale() > 1.25).then_some(options.clone()))
        .maybe_child((!focus && t::ui_scale() <= 1.25).then_some(activity.clone()));
    if focus {
        header = header.child(Command::Focus.button(writer));
    }
    let mut views = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_XS);
    if tools && !focus {
        views = views
            .child(Segments {
                shortcuts: Some(
                    [Command::Script, Command::Map, Command::Source].map(|c| c.shortcut(writer)),
                ),
                name: text(MsgId::WriterWorkspaceWritingView),
                labels: [
                    Command::Script.label(writer),
                    Command::Map.label(writer),
                    Command::Source.label(writer),
                ],
                ids,
                selected: match view {
                    WriterView::Script => 0,
                    WriterView::Map => 1,
                    WriterView::Source => 2,
                },
                vim: false,
                width: Size::px(240. * t::ui_scale()),
                change: EventHandler::new(move |index: usize| {
                    [Command::Script, Command::Map, Command::Source][index].run(writer)
                }),
            })
            .child(Command::Preview.button(writer));
    }
    let mut actions_row = rect()
        .horizontal()
        .content(Content::Flex)
        .spacing(t::SPACE_XS);
    if !focus {
        actions_row = actions_row
            .child(crate::controls::IconButton::new(
                "Undo",
                crate::controls::Icon::Undo,
                move || Command::Undo.run(writer),
            ))
            .child(crate::controls::IconButton::new(
                "Redo",
                crate::controls::Icon::Redo,
                move || Command::Redo.run(writer),
            ));
    }
    if !focus
        && !compact
        && let Some(actions) = actions
    {
        actions_row = actions_row.child(actions);
    }
    if one_row {
        return rect()
            .horizontal()
            .width(Size::fill())
            .content(Content::Flex)
            .cross_align(Alignment::Center)
            .padding((t::SPACE_SM, t::SPACE_LG))
            .spacing(t::SPACE_SM)
            .child(crate::navigation::buttons(writer))
            .child(title.max_width(Size::px(280. * t::ui_scale())))
            .child(rect().width(Size::flex(1.)))
            .child(views.width(Size::auto()).child(actions_row))
            .child(rect().width(Size::flex(1.)))
            .child(Command::Commands.button(writer))
            .child(activity)
            .child(options)
            .into_element();
    }
    views = views
        .child(actions_row)
        .child(rect().width(Size::flex(1.)))
        .maybe_child((t::ui_scale() <= 1.25).then_some(options));
    rect()
        .width(Size::fill())
        .padding((t::SPACE_SM, t::SPACE_LG))
        .spacing(t::SPACE_XS)
        .child(header)
        .maybe_child((!focus).then_some(views))
        .into_element()
}
