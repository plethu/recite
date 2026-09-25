//! Optional source context and trial entry point keep the translation form focused.
use super::{
    context_data::{Nearby, metadata},
    messages::{MsgId, text},
};
use crate::design::tokens::ProseTypography;
use crate::{
    design::{Button, tokens as t},
    editing::{Pane, Writer},
};
use freya::prelude::*;
use recite_core::po::{PoCommentKind, PoEntry};
#[derive(Clone)]
pub(super) struct EntryContext {
    pub writer: Writer,
    pub entry: PoEntry,
}
impl PartialEq for EntryContext {
    fn eq(&self, other: &Self) -> bool {
        self.entry == other.entry && self.writer.dark == other.writer.dark
    }
}
impl Component for EntryContext {
    fn render(&self) -> impl IntoElement {
        let mut open = use_state(|| false);
        let mut writer = self.writer;
        let entry = self.entry.clone();
        let passages = writer
            .buffers
            .model
            .peek()
            .as_ref()
            .ok()
            .and_then(|m| m.document().passages().ok())
            .unwrap_or_default();
        let destination = super::navigation::Destination::resolve(&entry, &passages);
        let mut body = rect().width(Size::fill()).spacing(t::SPACE_MD).child(
            crate::design::actions()
                .main_align(Alignment::Start)
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            let next = !*open.peek();
                            open.set(next);
                        })
                        .child(text(MsgId::WriterNearbySource)),
                )
                .child(
                    Button::new()
                        .flat()
                        .enabled(destination.is_some())
                        .on_press(move |_| {
                            let locale = writer
                                .localisation
                                .peek()
                                .catalogue
                                .as_ref()
                                .and_then(|c| {
                                    c.document
                                        .headers()
                                        .iter()
                                        .find(|h| h.key().eq_ignore_ascii_case("Language"))
                                        .map(|h| h.value().replace('_', "-"))
                                })
                                .unwrap_or_default();
                            writer.trial.locale.set(locale);
                            writer
                                .trial
                                .variant
                                .set(entry.variant().unwrap_or_default().into());
                            if let Some(destination) = &destination
                                && !destination.open(writer)
                            {
                                return;
                            }
                            writer.localisation.write().active = false;
                            writer.pane.set(Pane::Preview);
                        })
                        .child(text(MsgId::WriterPreview)),
                ),
        );
        if *open.read() {
            let entry = &self.entry;
            let caption = [
                metadata(entry, "file:"),
                metadata(entry, "block:"),
                metadata(entry, "speaker:"),
            ]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" · ");
            body = body.child(label().text(caption).font_size(t::small()));
            if let Some(template) = writer
                .buffers
                .model
                .read()
                .as_ref()
                .ok()
                .and_then(|m| m.document().extract_catalogue().catalog)
            {
                // PO serialization uses the same extracted metadata as source refresh.
                if let Ok(template) = recite_core::po::PoDocument::parse(template.to_pot_string()) {
                    for source in Nearby::new(&template).for_entry(entry) {
                        body = body
                            .child(label().text(source).font_size(t::prose_size()).prose_font());
                    }
                }
            }
            let notes: Vec<_> = entry
                .comments()
                .iter()
                .filter(|c| *c.kind() == PoCommentKind::Translator)
                .collect();
            if !notes.is_empty() {
                body = body.child(label().text(text(MsgId::WriterTranslatorNotes)));
                for note in notes {
                    body = body.child(label().text(note.text().to_owned()));
                }
            }
        }
        body
    }
}
