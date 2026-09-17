//! A bilingual manuscript over the same beat selection and ordinary PO files.
use messages::{MsgId, text as wording};
pub(crate) mod catalogue;
mod context;
mod create;
mod field;
mod messages;
mod navigation;
mod panel;
mod queue;
mod setup;
mod target;

use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use catalogue::Catalogue;
use freya::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Files,
    Create,
}

#[derive(Default)]
pub(crate) struct Localisation {
    pub active: bool,
    pub(crate) catalogue: Option<Catalogue>,
    panel: Option<Panel>,
    pub(crate) queue: bool,
    pub focus: Option<String>,
}
impl Localisation {
    pub fn translating(&self) -> bool {
        self.active && self.catalogue.is_some()
    }
    pub fn modal_open(&self) -> bool {
        self.active && self.panel.is_some()
    }
    pub fn dirty(&self) -> bool {
        self.catalogue.as_ref().is_some_and(Catalogue::dirty)
    }
}

pub(super) fn switch(writer: Writer) -> Element {
    let mut state = writer.localisation;
    let active = state.read().active;
    rect()
        .horizontal()
        .spacing(t::SPACE_XS)
        .child(
            Button::new()
                .flat()
                .selected(!active)
                .on_press(move |_| {
                    state.write().active = false;
                    writer.inspector_focus.request_focus();
                })
                .child(wording(MsgId::WriterWrite)),
        )
        .child(
            Button::new()
                .flat()
                .selected(active)
                .on_press(move |_| {
                    writer.navigate(|_| Ok(()));
                    if writer.message.peek().is_empty() {
                        state.write().active = true;
                        let first = writer
                            .buffers
                            .model
                            .peek()
                            .as_ref()
                            .ok()
                            .and_then(|session| {
                                session
                                    .selected_block()
                                    .ok()
                                    .flatten()
                                    .is_none()
                                    .then(|| session.document().sections().into_iter().next())
                                    .flatten()
                            });
                        if let Some(first) = first {
                            writer.inspect(&first);
                        }
                    }
                })
                .child(messages::mode_label(
                    &writer.preferences.read().config.ui.locale,
                )),
        )
        .into_element()
}

#[derive(Clone)]
pub(super) struct Surface {
    pub writer: Writer,
    pub files: State<Option<crate::project::ProjectFiles>>,
    pub reading: Element,
}
impl PartialEq for Surface {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.reading == other.reading
    }
}
impl Component for Surface {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let reading = self.reading.clone();
        let mut state = writer.localisation;
        let current = state.read();
        let mut header = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_SM)
            .padding(t::SPACE_SM);
        if let Some(catalogue) = &current.catalogue {
            let language = catalogue
                .document
                .headers()
                .iter()
                .find(|h| h.key() == "Language")
                .map_or_else(|| wording(MsgId::WriterCatalogue), |h| h.value().to_owned())
                .to_owned();
            header = header
                .child(label().text(language).font_size(t::TEXT_HEADING))
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| state.write().panel = Some(Panel::Files))
                        .child(
                            catalogue
                                .path
                                .file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .into_owned(),
                        ),
                )
                .child(rect().width(Size::flex(1.)))
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            let next = !state.peek().queue;
                            state.write().queue = next;
                        })
                        .child(wording(if current.queue {
                            MsgId::WriterReadPassage
                        } else {
                            MsgId::WriterTranslationQueue
                        })),
                );
        } else {
            header = header
                .child(label().text(wording(MsgId::WriterOptional)))
                .child(
                    Button::new()
                        .filled()
                        .on_press(move |_| {
                            writer.message.set(String::new());
                            state.write().panel = Some(Panel::Create);
                        })
                        .child(wording(MsgId::WriterStartLocalisation)),
                )
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| state.write().panel = Some(Panel::Files))
                        .child(wording(MsgId::WriterOpenCatalogue)),
                );
        }
        drop(current);
        rect()
            .a11y_id(writer.inspector_focus)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(wording(MsgId::WriterLocalisation))
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .child(header)
            .child(if state.read().queue {
                queue::QueueScreen { writer }.into_element()
            } else {
                ScrollView::new_controlled(writer.scroll)
                    .height(Size::flex(1.))
                    .width(Size::fill())
                    .child(reading)
                    .into_element()
            })
            .child(panel::render(writer))
            .child(setup::Setup {
                writer,
                files: self.files,
            })
            .into_element()
    }
}

pub(super) fn translation(writer: Writer, passage: recite_writer_model::Passage) -> Element {
    field::TranslationField { writer, passage }.into_element()
}

pub(crate) fn close_drafts_message() -> String {
    wording(MsgId::WriterCloseDrafts)
}

pub(super) fn context(writer: Writer, beat: String) -> Element {
    context::Context { writer, beat }.into_element()
}
