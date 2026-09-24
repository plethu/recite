use super::changes::Change;
use crate::design::tokens::ProseTypography;
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
        .child(label().text(change.caption.clone()).font_size(t::small()))
        .child(
            label()
                .text(wording(change.kind.label()))
                .font_size(t::heading()),
        );
    if !change.nearby.is_empty() {
        detail = detail.child(label().text(wording(MsgId::WriterNearbySource)));
        for text in &change.nearby {
            detail = detail.child(label().text(text.clone()));
        }
    }
    detail = detail.child(crate::design::ComparisonView {
        code: false,
        before: wording(MsgId::WriterPreviousSource),
        after: wording(MsgId::WriterCurrentSource),
        rows: vec![crate::design::ComparisonRow {
            caption: change.caption.clone(),
            before: change.old.clone(),
            after: change.new.clone(),
        }],
    });
    detail = detail
        .child(label().text(wording(MsgId::WriterTranslation)))
        .child(
            label()
                .text(if change.translation.is_empty() {
                    wording(MsgId::WriterUntranslated)
                } else {
                    change.translation.clone()
                })
                .font_size(t::prose_size())
                .prose_font(),
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
