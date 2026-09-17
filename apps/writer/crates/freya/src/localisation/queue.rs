//! A bounded queue; opening an entry retains the full beat manuscript.
use super::messages::{MsgId, text as wording};
use super::navigation::Destination;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct QueueScreen {
    pub writer: Writer,
}
impl PartialEq for QueueScreen {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for QueueScreen {
    fn render(&self) -> impl IntoElement {
        render(self.writer)
    }
}
fn render(writer: Writer) -> Element {
    let mut state = writer.localisation;
    let search = writer.queue.search;
    let mut attention = writer.queue.attention;
    let mut page = writer.queue.page;
    let search_id = use_a11y();
    let filter_id = use_a11y();
    let row_ids: [AccessibilityId; 34] = std::array::from_fn(|_| use_a11y());
    let mut row_ids = row_ids.into_iter();
    let query = search.read().trim().to_lowercase();
    let only_attention = *attention.read();
    let current = state.read();
    let mut content = rect()
        .spacing(t::SPACE_MD)
        .padding(t::SPACE_LG)
        .width(Size::fill())
        .child(
            label()
                .text(wording(MsgId::WriterTranslationQueue))
                .font_size(t::TEXT_TITLE),
        )
        .child(label().text(wording(MsgId::WriterQueueScope)))
        .child(
            Input::new(search)
                .a11y_id(search_id)
                .placeholder(wording(MsgId::WriterSearch))
                .on_validate(move |value: InputValidator| {
                    if *value.text() != *search.peek() {
                        page.set(0);
                    }
                })
                .width(Size::fill())
                .on_pre_key_down(crate::closing::text_input_key),
        )
        .child(crate::design::checkbox(
            filter_id,
            wording(MsgId::WriterAttention),
            only_attention,
            move |_| {
                let next = !*attention.peek();
                attention.set(next);
                page.set(0);
            },
        ));
    let model = writer.buffers.model.peek();
    if let (Some(catalogue), Ok(session)) = (&current.catalogue, model.as_ref()) {
        let passages = session.document().passage_snapshot().unwrap_or_default();
        let matches: Vec<_> = catalogue
            .document
            .entries()
            .iter()
            .filter(|entry| {
                if entry.is_header() || entry.is_obsolete() {
                    return false;
                }
                let draft = catalogue.draft(entry.id());
                (!only_attention || draft.as_ref().is_none_or(|d| !d.reviewed))
                    && (entry.source_text().to_lowercase().contains(&query)
                        || entry
                            .context()
                            .is_some_and(|id| id.to_lowercase().contains(&query))
                        || Destination::resolve(entry, &passages)
                            .is_some_and(|d| d.caption().to_lowercase().contains(&query))
                        || draft
                            .as_ref()
                            .is_some_and(|d| d.text.to_lowercase().contains(&query)))
            })
            .collect();
        let count = matches.len();
        content = content.child(label().text(format!(
            "{}: {count}",
            wording(MsgId::WriterMatchingEntries)
        )));
        let current_page = (*page.read()).min(count.saturating_sub(1) / 32);
        if count > 32 {
            let mut paging = rect().horizontal().spacing(t::SPACE_SM);
            for (caption, next, enabled) in [
                (
                    wording(MsgId::WriterPrevious),
                    current_page.saturating_sub(1),
                    current_page > 0,
                ),
                (
                    wording(MsgId::WriterNext),
                    current_page + 1,
                    (current_page + 1) * 32 < count,
                ),
            ] {
                let Some(id) = row_ids.next() else {
                    break;
                };
                paging = paging.child(
                    Button::new()
                        .a11y_id(id)
                        .enabled(enabled)
                        .on_press(move |_| page.set(next))
                        .child(caption),
                );
            }
            content = content.child(paging);
        }
        content = content.child(
            rect()
                .horizontal()
                .content(Content::Flex)
                .width(Size::fill())
                .spacing(t::SPACE_LG)
                .child(
                    label()
                        .width(Size::flex(1.))
                        .text(wording(MsgId::WriterSource))
                        .font_size(t::TEXT_SMALL),
                )
                .child(
                    label()
                        .width(Size::flex(1.))
                        .text(wording(MsgId::WriterTranslation))
                        .font_size(t::TEXT_SMALL),
                ),
        );
        for entry in matches.into_iter().skip(current_page * 32).take(32) {
            let destination = Destination::resolve(entry, &passages);
            let draft = catalogue.draft(entry.id());
            let translation = draft.as_ref().map_or("", |d| d.text.as_str());
            let status = if translation.is_empty() {
                wording(MsgId::WriterUntranslated)
            } else if draft.as_ref().is_some_and(|d| d.reviewed) {
                wording(MsgId::WriterReviewed)
            } else {
                wording(MsgId::WriterAttention)
            };
            let caption = destination.as_ref().map_or_else(
                || wording(MsgId::WriterUnavailablePassage),
                Destination::caption,
            );
            let Some(id) = row_ids.next() else {
                break;
            };
            content = content.child(
                Button::new()
                    .flat()
                    .a11y_id(id)
                    .width(Size::fill())
                    .enabled(destination.is_some())
                    .on_press(move |_| {
                        if let Some(destination) = &destination
                            && destination.open(writer)
                        {
                            state.write().queue = false;
                            writer.inspector_focus.request_focus();
                        }
                    })
                    .child(
                        rect()
                            .width(Size::fill())
                            .child(
                                rect()
                                    .horizontal()
                                    .content(Content::Flex)
                                    .width(Size::fill())
                                    .spacing(t::SPACE_LG)
                                    .child(label().width(Size::flex(1.)).text(
                                        entry.source_text().chars().take(180).collect::<String>(),
                                    ))
                                    .child(label().width(Size::flex(1.)).text(
                                        if translation.trim().is_empty() {
                                            wording(MsgId::WriterUntranslated)
                                        } else {
                                            translation.chars().take(180).collect::<String>()
                                        },
                                    )),
                            )
                            .child(
                                label()
                                    .text(format!("{caption} · {status}"))
                                    .font_size(t::TEXT_SMALL),
                            ),
                    ),
            );
        }
        if count == 0 {
            content = content.child(label().text(wording(MsgId::WriterNoMatches)));
        }
    }
    ScrollView::new()
        .width(Size::fill())
        .height(Size::flex(1.))
        .child(content)
        .into_element()
}
