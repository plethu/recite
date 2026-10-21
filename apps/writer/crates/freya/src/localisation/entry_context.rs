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
use recite_writer_session::localisation::target::document_language;
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
            .and_then(|m| m.document().cached_passages())
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
                                .map(|catalogue| document_language(&catalogue.document))
                                .transpose();
                            let locale = match locale {
                                Ok(locale) => locale
                                    .flatten()
                                    .map(|locale| locale.to_string())
                                    .unwrap_or_default(),
                                Err(error) => {
                                    writer.message.error(error);
                                    return;
                                }
                            };
                            let variant = entry.variant().unwrap_or_default().to_owned();
                            let completed = preview_completed(writer, locale, variant);
                            if let Some(destination) = &destination {
                                super::navigation::open(destination, writer, completed);
                            } else {
                                completed.call(Ok(()));
                            }
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

fn preview_completed(
    mut writer: Writer,
    locale: String,
    variant: String,
) -> EventHandler<Result<(), String>> {
    EventHandler::new(move |result: Result<(), String>| {
        if let Err(error) = result {
            writer.message.error(error);
            return;
        }
        writer.trial.locale.set(locale.clone());
        writer.trial.variant.set(variant.clone());
        writer.localisation.write().active = false;
        writer.pane.set(Pane::Preview);
    })
}

#[cfg(test)]
mod tests;
