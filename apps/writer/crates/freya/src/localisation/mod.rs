//! A bilingual manuscript over the same beat selection and ordinary PO files.
pub(crate) mod watching;
use messages::{MsgId, text as wording};
pub(crate) mod catalogue;
mod close_drafts;
mod comparison;
pub(crate) use close_drafts::CloseDrafts;
mod context;
mod manuscript;
pub(super) use manuscript::{columns, source_passage};
mod create;
mod entry;
mod extraction;
mod field;
mod language_picker;
pub(crate) mod messages;
mod navigation;
mod panel;
mod queue;
pub(crate) mod refresh;
mod setup;
pub(crate) use language_picker::LanguagePicker;
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum CatalogueView {
    #[default]
    Passage,
    Queue,
    Updates,
    Compare,
    Entry,
}

#[derive(Default)]
pub(crate) struct Localisation {
    pub active: bool,
    pub(crate) catalogue: Option<Catalogue>,
    panel: Option<Panel>,
    pub(crate) view: CatalogueView,
    pub(crate) refresh: Option<refresh::Preview>,
    pub(crate) update_index: usize,
    pub focus: Option<String>,
    pub(crate) entry_context: Option<String>,
    comparison: Option<catalogue::Comparison>,
    comparison_return: CatalogueView,
}
impl Localisation {
    pub(crate) fn install(&mut self, catalogue: Option<Catalogue>) -> Result<(), String> {
        if self.dirty() {
            return Err(
                "Save or discard translation drafts before opening another catalogue.".into(),
            );
        }
        if let Some(current) = self.catalogue.as_mut() {
            current.flush_recovery()?;
        }
        self.catalogue = catalogue;
        self.refresh = None;
        self.comparison = None;
        self.entry_context = None;
        self.view = CatalogueView::Passage;
        Ok(())
    }
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
    let mut pane = writer.pane;
    let active = state.read().active;
    crate::design::Segments {
        name: "Activity".into(),
        labels: [
            wording(MsgId::WriterWrite),
            messages::mode_label(&writer.preferences.read().config.ui.locale),
        ],
        ids: [use_a11y(), use_a11y()],
        selected: usize::from(active),
        vim: false,
        width: Size::px(172.),
        change: EventHandler::new(move |index| {
            if index == 0 {
                state.write().active = false;
                if *pane.peek() == crate::editing::Pane::Preview {
                    pane.set(crate::editing::Pane::Script);
                }
                writer.inspector_focus.request_focus();
            } else {
                let _ = enter(writer);
            }
        }),
    }
    .into_element()
}

/// Enter through the selected beat, whether invoked by the switch or Commands.
pub(crate) fn enter(mut writer: Writer) -> Result<(), String> {
    writer.try_navigate(|_| Ok(()))?;
    let beat = writer.selection.peek().clone().or_else(|| {
        writer
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
                    .or_else(|| session.document().sections().into_iter().next())
            })
    });
    if let Some(beat) = beat {
        writer.try_navigate(|m| m.inspect_block(&beat))?;
        writer.selection.set(Some(beat));
    }
    writer.localisation.write().active = true;
    writer.pane.set(crate::editing::Pane::Script);
    writer.inspector_focus.request_focus();
    Ok(())
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
                .child(
                    label()
                        .text(setup::languages::caption(&language).unwrap_or(language))
                        .font_size(t::heading()),
                )
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
                            let next = if state.peek().view == CatalogueView::Passage {
                                CatalogueView::Queue
                            } else {
                                CatalogueView::Passage
                            };
                            state.write().view = next;
                        })
                        .child(wording(if current.view != CatalogueView::Passage {
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
                            writer.message.clear();
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
        let updates = current.view == CatalogueView::Updates;
        drop(current);
        rect()
            .a11y_id(writer.inspector_focus)
            .a11y_focusable(true)
            .a11y_role(AccessibilityRole::Group)
            .a11y_alt(wording(MsgId::WriterLocalisation))
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .maybe_child((!updates).then_some(header))
            .maybe_child(
                (state.read().view == CatalogueView::Passage).then(|| manuscript::header(writer)),
            )
            .child(if updates {
                refresh::screen::RefreshScreen {
                    writer,
                    files: self.files,
                }
                .into_element()
            } else if state.read().view == CatalogueView::Compare {
                comparison::ExternalComparison { writer }.into_element()
            } else if state.read().view == CatalogueView::Entry {
                entry::EntryEditor { writer }.into_element()
            } else if state.read().view == CatalogueView::Queue {
                queue::QueueScreen { writer }.into_element()
            } else {
                ScrollView::new_controlled(writer.scroll)
                    .height(Size::flex(1.))
                    .width(Size::fill())
                    .child(reading)
                    .into_element()
            })
            .child(panel::render(writer, self.files))
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

mod status;

mod context_data;
mod entry_context;

pub(crate) fn close_drafts_message() -> String {
    wording(MsgId::WriterCloseDrafts)
}

pub(crate) fn show_unsaved(mut writer: Writer) {
    let mut state = writer.localisation.write();
    let context = state.catalogue.as_ref().and_then(|c| {
        c.document
            .entries()
            .iter()
            .find(|e| c.changed(e.id()))
            .and_then(|e| e.context())
            .map(str::to_owned)
    });
    state.panel = None;
    state.active = true;
    state.view = if context.is_some() {
        CatalogueView::Entry
    } else {
        CatalogueView::Queue
    };
    state.entry_context = context;
    writer.message.clear();
}
