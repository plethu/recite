use super::catalogue::Draft;
use super::messages::{MsgId, text as wording};
use crate::design::tokens::ProseTypography;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
use recite_writer_model::Passage;

#[derive(Clone)]
pub(super) struct TranslationField {
    pub writer: Writer,
    pub passage: Passage,
}
impl PartialEq for TranslationField {
    fn eq(&self, other: &Self) -> bool {
        self.passage == other.passage && self.writer.dark == other.writer.dark
    }
}
impl Component for TranslationField {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut state = writer.localisation;
        let mut message = writer.message;
        let input_id = use_a11y();
        let review_id = use_a11y();
        let anchor = self.passage.id.clone();
        use_side_effect(move || {
            if input_id.is_focused() {
                state.write().focus = Some(anchor.clone());
                let selected =
                    writer.buffers.model.peek().as_ref().is_ok_and(|m| {
                        m.view() == &recite_writer_model::View::Passage(anchor.clone())
                    });
                if !selected {
                    writer
                        .navigate(|m| m.select(recite_writer_model::View::Passage(anchor.clone())));
                }
            }
        });
        let mut text = use_state(String::new);
        let mut identity = use_state(|| None);
        let current = state.read();
        let Some(catalogue) = &current.catalogue else {
            return rect().into_element();
        };
        let Some(entry_id) = catalogue.entry_for(&self.passage.id, &self.passage.text) else {
            return rect()
                .width(Size::fill())
                .child(
                    label()
                        .text(wording(MsgId::WriterNoEntry))
                        .font_size(t::small())
                        .color(palette::muted(writer.dark)),
                )
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            state.write().view = super::CatalogueView::Updates;
                            state.write().refresh = None;
                        })
                        .child(wording(MsgId::WriterSourceUpdates)),
                )
                .into_element();
        };
        let Some(draft) = catalogue.draft(entry_id) else {
            return rect().into_element();
        };
        let changed = catalogue.changed(entry_id);
        let next_identity = (
            catalogue.path.clone(),
            entry_id,
            draft.forms.first().cloned().unwrap_or_default(),
        );
        if identity.peek().as_ref() != Some(&next_identity) {
            text.set_if_modified(draft.forms.first().cloned().unwrap_or_default());
            identity.set(Some(next_identity));
        }
        let reviewed = draft.reviewed;
        let status = super::status::TranslationStatus::entry_label(catalogue, entry_id);
        drop(current);
        let save = move || {
            let result = state.write().catalogue.as_mut().map(|c| c.save(entry_id));
            if let Some(result) = result {
                message.report(result, wording(MsgId::WriterTranslationSaved));
            }
        };
        let mut save_key = save;
        let mut save_button = save;
        let mut body = rect()
            .width(Size::fill())
            .spacing(t::SPACE_XS)
            .child(
                rect()
                    .height(Size::px(t::prose_meta_height()))
                    .cross_align(Alignment::Center)
                    .child(
                        label()
                            .text(status)
                            .font_size(t::small())
                            .color(palette::muted(writer.dark)),
                    ),
            )
            .child(
                rect().prose_font().font_size(t::heading()).child(
                    Input::new(text)
                        .a11y_id(input_id)
                        .multiline(true)
                        .width(Size::fill())
                        .height(Size::Inner)
                        .placeholder(wording(MsgId::WriterPlaceholder))
                        .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                            if crate::editing::is_save_key(&event) {
                                event.stop_propagation();
                                event.prevent_default();
                                save_key();
                                false
                            } else {
                                crate::closing::text_input_key(event)
                            }
                        })
                        .on_validate(move |value: InputValidator| {
                            if let Some(catalogue) = state.write().catalogue.as_mut() {
                                catalogue.update(
                                    entry_id,
                                    Draft {
                                        forms: vec![value.text().clone()],
                                        reviewed: false,
                                    },
                                );
                            }
                        }),
                ),
            );
        let mut footer = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .cross_align(Alignment::Center)
            .spacing(t::SPACE_XS);
        if !draft.forms.iter().all(|text| text.trim().is_empty()) {
            footer = footer.child(rect().width(Size::flex(1.)).child(crate::design::checkbox(
                review_id,
                wording(MsgId::WriterReviewed),
                reviewed,
                move |_| {
                    if let Some(catalogue) = state.write().catalogue.as_mut()
                        && let Some(mut draft) = catalogue.draft(entry_id)
                    {
                        draft.reviewed = !draft.reviewed;
                        catalogue.update(entry_id, draft);
                    }
                },
            )));
        } else {
            footer = footer.child(rect().width(Size::flex(1.)));
        }
        if changed {
            footer = footer
                .child(
                    Button::new()
                        .filled()
                        .on_press(move |_| save_button())
                        .child(wording(MsgId::WriterSave)),
                )
                .child(
                    Button::new()
                        .flat()
                        .on_press(move |_| {
                            if let Some(catalogue) = state.write().catalogue.as_mut() {
                                catalogue.discard(entry_id);
                            }
                        })
                        .child(wording(MsgId::WriterDiscard)),
                );
        }
        body = body.child(footer);
        body.into_element()
    }
}
