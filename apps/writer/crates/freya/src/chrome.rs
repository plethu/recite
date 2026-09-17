//! Controls colocated with the active scene editor.
use crate::design::Button;
use crate::design::tokens as t;
use crate::{controls, editing::Writer, palette};
use freya::prelude::*;
use recite_writer_model::{View, Workbench};

pub(super) fn toolbar(writer: Writer, actions: Option<Element>) -> Element {
    let state = writer.buffers.model.peek();
    let Ok(session) = state.as_ref() else {
        return rect().into_element();
    };
    let source = session.view() == &View::Source;
    let scene_name = palette::display_name(session.document().key().as_str());
    let header = rect()
        .horizontal()
        .content(Content::Flex)
        .width(Size::fill())
        .padding((8., 16.))
        .cross_align(Alignment::Center)
        .child(crate::navigation::buttons(writer))
        .child(label().text(scene_name).font_size(t::TEXT_HEADING))
        .child(rect().width(Size::flex(1.)))
        .child(crate::localisation::switch(writer));
    if writer.localisation.read().active {
        return header.into_element();
    }
    let mut pane = writer.pane;
    let mut toolbar = rect()
        .width(Size::fill())
        .horizontal()
        .content(Content::Flex)
        .cross_align(Alignment::Center)
        .spacing(t::SPACE_SM)
        .padding((8., 16.))
        .child(rect().width(Size::flex(1.)))
        .child(
            Button::new()
                .flat()
                .selected(!source)
                .on_press(move |_| {
                    writer.set_view(recite_config::WriterView::Map);
                })
                .child("Map"),
        )
        .child(
            Button::new()
                .flat()
                .selected(source)
                .on_press(move |_| writer.set_view(recite_config::WriterView::Source))
                .child("Source"),
        )
        .child(controls::IconButton::new(
            "Undo",
            controls::Icon::Undo,
            move || writer.navigate(Workbench::undo),
        ))
        .child(controls::IconButton::new(
            "Redo",
            controls::Icon::Redo,
            move || writer.navigate(Workbench::redo),
        ))
        .child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    writer.navigate(Workbench::start_preview);
                    if writer
                        .buffers
                        .model
                        .peek()
                        .as_ref()
                        .is_ok_and(|m| m.preview_page().is_some())
                    {
                        pane.set(crate::editing::Pane::Preview);
                    }
                })
                .child("Try scene"),
        );
    if let Some(actions) = actions {
        toolbar = toolbar.child(actions);
    }
    rect()
        .width(Size::fill())
        .child(header)
        .child(toolbar)
        .into_element()
}
