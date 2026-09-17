use super::{changes::Change, diff};
use crate::localisation::messages::{MsgId, text as wording};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
pub(super) fn render(change: &Change, writer: Writer) -> Element {
    let mut detail = rect()
        .width(Size::fill())
        .spacing(t::SPACE_LG)
        .child(
            label()
                .text(change.caption.clone())
                .font_size(t::TEXT_SMALL),
        )
        .child(
            label()
                .text(wording(change.kind.label()))
                .font_size(t::TEXT_HEADING),
        );
    if !change.nearby.is_empty() {
        detail = detail.child(label().text(wording(MsgId::WriterNearbySource)));
        for text in &change.nearby {
            detail = detail.child(label().text(text.clone()));
        }
    }
    for (title, text, other) in [
        (MsgId::WriterPreviousSource, &change.old, &change.new),
        (MsgId::WriterCurrentSource, &change.new, &change.old),
    ] {
        if let Some(text) = text {
            detail = detail
                .child(label().text(wording(title)))
                .child(diff::render(text, other.as_deref().unwrap_or("")));
        }
    }
    detail = detail
        .child(label().text(wording(MsgId::WriterTranslation)))
        .child(
            label()
                .text(if change.translation.is_empty() {
                    wording(MsgId::WriterUntranslated)
                } else {
                    change.translation.clone()
                })
                .font_size(t::TEXT_PROSE)
                .font_family("serif"),
        );
    if !change.notes.is_empty() {
        detail = detail.child(label().text(wording(MsgId::WriterTranslatorNotes)));
        for note in &change.notes {
            detail = detail.child(label().text(note.clone()));
        }
    }
    if let Some(destination) = change.destination.clone() {
        detail = detail.child(
            Button::new()
                .flat()
                .on_press(move |_| {
                    destination.open(writer);
                })
                .child(wording(MsgId::WriterReadPassage)),
        );
    }
    detail.into_element()
}
