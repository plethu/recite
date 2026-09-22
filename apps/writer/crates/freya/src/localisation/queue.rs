//! A bounded queue; opening an entry retains the full beat manuscript.
use super::messages::{MsgId, text as wording};
use super::navigation::Destination;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    palette,
};
use freya::prelude::*;
use t::ProseTypography;

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
    let state = writer.localisation;
    let search = writer.queue.search;
    let mut attention = writer.queue.attention;
    let mut page = writer.queue.page;
    let search_id = use_a11y();
    let mut active = use_state(|| None::<usize>);
    let filter_id = use_a11y();
    let row_ids: [AccessibilityId; 34] = std::array::from_fn(|_| use_a11y());
    let mut row_ids = row_ids.into_iter();
    let query = search.read().trim().to_lowercase();
    let only_attention = *attention.read();
    let current = state.read();
    let mut content = rect()
        .spacing(t::SPACE_SM)
        .padding(t::SPACE_LG)
        .width(Size::fill())
        .child(
            label()
                .text(wording(MsgId::WriterTranslationQueue))
                .font_size(t::title()),
        );
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
                (!only_attention
                    || super::status::TranslationStatus::for_entry(catalogue, entry.id())
                        .needs_attention())
                    && (entry.source_text().to_lowercase().contains(&query)
                        || entry
                            .context()
                            .is_some_and(|id| id.to_lowercase().contains(&query))
                        || Destination::resolve(entry, &passages).is_some_and(|d| {
                            d.caption()
                                .to_lowercase()
                                .contains(&query.replace('_', " "))
                        })
                        || draft.as_ref().is_some_and(|d| {
                            d.forms
                                .iter()
                                .any(|text| text.to_lowercase().contains(&query))
                        }))
            })
            .collect();
        let count = matches.len();
        let current_page = (*page.read()).min(count.saturating_sub(1) / 32);
        let destinations: Vec<_> = matches
            .iter()
            .skip(current_page * 32)
            .take(32)
            .map(|entry| entry.context().map(str::to_owned))
            .collect();
        content = content
            .child(crate::design::SearchField {
                query: search,
                id: search_id,
                placeholder: wording(MsgId::WriterSearch),
                active,
                count: destinations.len(),
                vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                changed: EventHandler::new(move |()| page.set(0)),
                activate: EventHandler::new(move |index: usize| {
                    if let Some(Some(destination)) = destinations.get(index) {
                        open(writer, destination);
                    }
                }),
            })
            .child(
                rect()
                    .horizontal()
                    .width(Size::fill())
                    .content(Content::Flex)
                    .cross_align(Alignment::Center)
                    .child(rect().width(Size::flex(1.)).child(crate::design::checkbox(
                        filter_id,
                        wording(MsgId::WriterAttention),
                        only_attention,
                        move |_| {
                            let next = !*attention.peek();
                            attention.set(next);
                            page.set(0);
                            active.set(None);
                        },
                    )))
                    .child(
                        label()
                            .text(format!(
                                "{}: {count}",
                                wording(MsgId::WriterMatchingEntries)
                            ))
                            .text_align(TextAlign::Right),
                    ),
            );
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
                        .on_press(move |_| {
                            page.set(next);
                            active.set(None);
                        })
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
                        .font_size(t::small()),
                )
                .child(
                    label()
                        .width(Size::flex(1.))
                        .text(wording(MsgId::WriterTranslation))
                        .font_size(t::small()),
                ),
        );
        let mut rows = rect().width(Size::fill());
        for (index, entry) in matches
            .into_iter()
            .skip(current_page * 32)
            .take(32)
            .enumerate()
        {
            let destination = Destination::resolve(entry, &passages);
            let context = entry.context().map(str::to_owned);
            let draft = catalogue.draft(entry.id());
            let translation = draft
                .as_ref()
                .map_or_else(String::new, |d| d.forms.join(" · "));
            let status = super::status::TranslationStatus::entry_label(catalogue, entry.id());
            let caption = destination.as_ref().map_or_else(
                || wording(MsgId::WriterUnavailablePassage),
                Destination::caption,
            );
            let Some(id) = row_ids.next() else {
                break;
            };
            rows = rows.child(
                rect()
                    .width(Size::fill())
                    .border(
                        Border::new()
                            .width(BorderWidth {
                                bottom: 1.,
                                ..Default::default()
                            })
                            .fill(palette::rule(writer.dark)),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .a11y_id(id)
                            .selected(*active.read() == Some(index))
                            .width(Size::fill())
                            .enabled(context.is_some())
                            .on_press(move |_| {
                                if let Some(context) = &context {
                                    open(writer, context);
                                }
                            })
                            .child(
                                rect()
                                    .horizontal()
                                    .content(Content::Flex)
                                    .width(Size::fill())
                                    .padding((t::SPACE_SM, 0.))
                                    .spacing(t::SPACE_LG)
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .spacing(t::SPACE_XS)
                                            .child(
                                                label()
                                                    .width(Size::fill())
                                                    .text(
                                                        entry
                                                            .source_text()
                                                            .chars()
                                                            .take(180)
                                                            .collect::<String>(),
                                                    )
                                                    .prose_font()
                                                    .font_size(t::heading()),
                                            )
                                            .child(
                                                label()
                                                    .text(format!("{caption}  →"))
                                                    .font_size(t::small())
                                                    .color(palette::muted(writer.dark)),
                                            ),
                                    )
                                    .child(
                                        rect()
                                            .width(Size::flex(1.))
                                            .spacing(t::SPACE_XS)
                                            .maybe_child((!translation.trim().is_empty()).then(
                                                || {
                                                    label()
                                                        .width(Size::fill())
                                                        .text(
                                                            translation
                                                                .chars()
                                                                .take(180)
                                                                .collect::<String>(),
                                                        )
                                                        .prose_font()
                                                        .font_size(t::heading())
                                                },
                                            ))
                                            .child(
                                                label()
                                                    .text(status)
                                                    .font_size(t::small())
                                                    .color(palette::muted(writer.dark)),
                                            ),
                                    ),
                            ),
                    ),
            );
        }
        content = content.child(rows);
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

fn open(writer: Writer, context: &str) {
    let mut state = writer.localisation;
    let mut current = state.write();
    current.entry_context = Some(context.to_owned());
    current.view = super::CatalogueView::Entry;
    writer.inspector_focus.request_focus();
}
