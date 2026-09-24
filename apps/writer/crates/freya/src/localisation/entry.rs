//! Whole-entry editing: variants own their drafts; plural arms share review.
use super::{
    CatalogueView,
    messages::{MsgId, text},
};
use crate::design::tokens::ProseTypography;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use recite_core::PoEntryId;

#[derive(Clone)]
pub(super) struct EntryEditor {
    pub writer: Writer,
}
impl PartialEq for EntryEditor {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for EntryEditor {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut state = writer.localisation;
        let review_id = use_a11y();
        let save_id = use_a11y();
        let mut rule_open = use_state(|| false);
        let current = state.read();
        let mut body = rect()
            .width(Size::fill())
            .max_width(Size::px(960.))
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .child(
                label()
                    .text(text(MsgId::WriterEntryEditor))
                    .font_size(t::title()),
            )
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| state.write().view = CatalogueView::Queue)
                    .child(text(MsgId::WriterTranslationQueue)),
            );
        let Some(catalogue) = &current.catalogue else {
            return body.into_element();
        };
        let Some(context) = &current.entry_context else {
            return body.into_element();
        };
        let mut entries = catalogue
            .document
            .entries()
            .iter()
            .filter(|entry| !entry.is_obsolete() && entry.context() == Some(context));
        let Some(entry) = entries.next().filter(|_| entries.next().is_none()) else {
            return body
                .child(label().text(text(MsgId::WriterNoEntry)))
                .into_element();
        };
        let id = entry.id();
        let Some(draft) = catalogue.draft(id) else {
            return body.into_element();
        };
        let plural = entry.is_plural();
        let rule = catalogue.plural_rule().map(str::to_owned);
        let valid = !plural
            || rule
                .as_deref()
                .and_then(|rule| recite_core::validate_plural_rule(rule).ok())
                == Some(draft.forms.len());
        let changed = catalogue.changed(id);
        let primary = crate::design::SubmitAction {
            id: save_id,
            caption: text(MsgId::WriterSave),
            enabled: changed && valid,
            action: EventHandler::new(move |()| save(writer, id)),
        };
        let shortcut = primary.clone();
        body = body.on_global_key_down(move |event: Event<KeyboardEventData>| {
            if !state.peek().modal_open() && crate::design::keyboard::submit_key(&event) {
                event.prevent_default();
                event.stop_propagation();
                shortcut.run();
            }
        });
        let root = context.split('&').next().unwrap_or(context);
        let mut variants = crate::design::actions().main_align(Alignment::Start);
        for variant in catalogue.document.entries().iter().filter(|e| {
            !e.is_obsolete()
                && e.context()
                    .is_some_and(|c| c.split('&').next() == Some(root))
        }) {
            let next = variant.context().unwrap_or_default().to_owned();
            let caption = variant
                .variant()
                .map_or_else(|| text(MsgId::WriterDefaultWording), str::to_owned);
            variants = variants.child(
                Button::new()
                    .flat()
                    .selected(variant.id() == id)
                    .on_press(move |_| state.write().entry_context = Some(next.clone()))
                    .child(caption),
            );
        }
        body = body.child(
            label()
                .text(entry.source_text().to_owned())
                .prose_font()
                .font_size(t::prose_size()),
        );
        if let Some(source) = entry.plural_source_text() {
            body = body.child(
                label()
                    .text(source.to_owned())
                    .prose_font()
                    .font_size(t::prose_size()),
            );
        }
        body = body.child(super::entry_context::EntryContext {
            writer,
            entry: entry.clone(),
        });
        body = body.child(variants);
        for (index, value) in draft.forms.iter().enumerate() {
            let examples: Vec<_> = rule
                .as_deref()
                .map(|rule| {
                    (0..=200)
                        .filter(|count| {
                            recite_core::evaluate_plural_form(rule, *count).ok() == Some(index)
                        })
                        .take(5)
                        .map(|count| count.to_string())
                        .collect()
                })
                .unwrap_or_default();
            let caption = if plural {
                format!(
                    "{} {} · {}",
                    text(MsgId::WriterPluralForm),
                    index + 1,
                    examples.join(", ")
                )
            } else {
                text(MsgId::WriterTranslation)
            };
            body = body.child(rect().key((id, index)).width(Size::fill()).child(Arm {
                writer,
                id,
                index,
                value: value.clone(),
                caption,
            }));
        }
        if plural {
            body = body.child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        let next = !*rule_open.peek();
                        rule_open.set(next);
                    })
                    .child(text(MsgId::WriterPluralRule)),
            );
            if *rule_open.read() || !valid {
                body = body.child(
                    label().text(rule.unwrap_or_else(|| text(MsgId::WriterEntryPluralInvalid))),
                );
            }
        }
        body = body.child(crate::design::checkbox(
            review_id,
            text(MsgId::WriterReviewed),
            draft.reviewed,
            move |_| {
                if let Some(catalogue) = state.write().catalogue.as_mut()
                    && let Some(mut draft) = catalogue.draft(id)
                {
                    draft.reviewed = !draft.reviewed;
                    catalogue.update(id, draft);
                }
            },
        ));
        body = body.child(
            crate::design::actions()
                .main_align(Alignment::Start)
                .child(primary.button())
                .child(
                    Button::new()
                        .flat()
                        .enabled(changed)
                        .on_press(move |_| {
                            if let Some(c) = state.write().catalogue.as_mut() {
                                c.discard(id);
                            }
                        })
                        .child(text(MsgId::WriterDiscard)),
                )
                .child(
                    label()
                        .text(super::status::TranslationStatus::for_entry(catalogue, id).label()),
                ),
        );
        ScrollView::new()
            .width(Size::fill())
            .height(Size::flex(1.))
            .child(body)
            .into_element()
    }
}

#[derive(Clone)]
struct Arm {
    writer: Writer,
    id: PoEntryId,
    index: usize,
    value: String,
    caption: String,
}
impl PartialEq for Arm {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.index == other.index
            && self.value == other.value
            && self.caption == other.caption
    }
}
impl Component for Arm {
    fn render(&self) -> impl IntoElement {
        let mut value = use_state(String::new);
        value.set_if_modified(self.value.clone());
        let writer = self.writer;
        let mut state = writer.localisation;
        let id = self.id;
        let index = self.index;
        rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text(self.caption.clone()))
            .child(
                Input::new(value)
                    .multiline(true)
                    .width(Size::fill())
                    .height(Size::px(96.))
                    .on_validate(move |input: InputValidator| {
                        if let Some(catalogue) = state.write().catalogue.as_mut()
                            && let Some(mut draft) = catalogue.draft(id)
                            && let Some(form) = draft.forms.get_mut(index)
                            && *form != *input.text()
                        {
                            *form = input.text().clone();
                            draft.reviewed = false;
                            catalogue.update(id, draft);
                        }
                    })
                    .on_pre_key_down(move |event: Event<KeyboardEventData>| {
                        if crate::editing::is_save_key(&event) {
                            event.prevent_default();
                            event.stop_propagation();
                            save(writer, id);
                            false
                        } else {
                            crate::closing::text_input_key(event)
                        }
                    }),
            )
    }
}
fn save(mut writer: Writer, id: PoEntryId) {
    let result = writer
        .localisation
        .write()
        .catalogue
        .as_mut()
        .map(|c| c.save(id));
    if let Some(result) = result {
        writer
            .message
            .report(result, text(MsgId::WriterTranslationSaved));
    }
}
