//! Bounded project search results retain document and beat context.
use crate::navigation::location;
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;

#[derive(Clone)]
struct SearchSnapshot(Option<std::sync::Weak<recite_writer_model::SearchIndex>>);
impl PartialEq for SearchSnapshot {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => std::sync::Weak::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        }
    }
}

#[derive(Clone)]
pub(super) struct ProjectSearch {
    pub writer: Writer,
    pub files: State<Option<ProjectFiles>>,
}
impl PartialEq for ProjectSearch {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.files == other.files
    }
}
impl Component for ProjectSearch {
    fn render(&self) -> impl IntoElement {
        let query = use_state(String::new);
        let mut limit = use_state(|| 100usize);
        let active = use_state(|| None::<usize>);
        let id = use_a11y();
        let scroll = use_scroll_controller(ScrollConfig::default);
        let row_height = 84. * t::ui_scale();
        crate::design::use_list_reveal(Some(query), active, scroll, row_height, 1);
        let writer = self.writer;
        let files = self.files;
        // Identity invalidates results without retaining the old project-sized index.
        let index = use_memo(move || {
            SearchSnapshot(
                files
                    .read()
                    .as_ref()
                    .map(|files| std::sync::Arc::downgrade(&files.search_index())),
            )
        });
        let results = use_memo(move || {
            index
                .read()
                .0
                .as_ref()
                .and_then(std::sync::Weak::upgrade)
                .map(|index| index.search_results(&query.read(), *limit.read()))
                .unwrap_or_default()
        });
        let hits = &*results.read();
        let total = hits.total();
        let mut content =
            rect()
                .width(Size::fill())
                .spacing(t::SPACE_XS)
                .child(crate::design::SearchField {
                    insert_request: None,
                    query,
                    id,
                    placeholder: "Search project words…".into(),
                    active,
                    count: hits.len(),
                    vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                    changed: EventHandler::new(move |()| limit.set(100)),
                    activate: EventHandler::new(move |index: usize| {
                        let snapshot = results.peek().clone();
                        if let Some(hit) = snapshot.get(index) {
                            open_result(writer, files, hit);
                        }
                    }),
                });
        if !query.read().trim().is_empty() {
            let list = VirtualScrollView::new_with_data(
                (hits.clone(), *active.read(), query.read().clone()),
                move |item, (hits, active, query)| {
                    rect()
                        .key(item.index)
                        .width(Size::fill())
                        .height(Size::px(row_height))
                        .overflow(Overflow::Clip)
                        .child(result_row(
                            writer,
                            files,
                            hits.clone(),
                            query,
                            item.index,
                            *active == Some(item.index),
                        ))
                        .into_element()
                },
            )
            .scroll_controller(scroll)
            .length(hits.len())
            .item_size(row_height)
            .height(Size::px(200. * t::ui_scale()))
            .width(Size::fill());
            content = content
                .child(
                    label()
                        .text(format!("{} of {total} saved lines", hits.len()))
                        .font_size(t::small()),
                )
                .child(list);
            if hits.is_empty() {
                content = content.child(label().text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiNoMatchingSavedPassagesTryFewerWords,
                )));
            }
            if hits.len() < total {
                content = content.child(
                    Button::new()
                        .on_press(move |_| {
                            let next = *limit.peek() + 100;
                            limit.set(next);
                        })
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiShowMoreResults,
                        )),
                );
            }
        }
        content
    }
}

#[cfg(test)]
mod tests;

fn result_row(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
    results: recite_writer_model::SearchResults,
    query: &str,
    index: usize,
    active: bool,
) -> Element {
    let Some(hit) = results.get(index) else {
        return rect().into_element();
    };
    let mut name = format!("{} · {} · {}", hit.document, hit.beat, hit.text);
    let caption = format!(
        "{} · {}",
        crate::palette::display_name(&hit.beat),
        hit.document
    );
    if !hit.speaker.is_empty() {
        name.push_str(&format!(" · {}", hit.speaker));
    }
    let excerpt = hit.excerpt(query, 90);
    let speaker = (!hit.speaker.is_empty()).then(|| hit.speaker.to_string());
    Button::new()
        .flat()
        .width(Size::fill())
        .toggle(active)
        .named(name)
        .on_press(move |_| {
            if let Some(hit) = results.get(index) {
                open_result(writer, files, hit);
            }
        })
        .child(
            rect()
                .width(Size::fill())
                .child(
                    label()
                        .text(caption)
                        .font_size(t::small())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis),
                )
                .maybe_child(speaker.map(|speaker| {
                    label()
                        .text(speaker)
                        .font_size(t::small())
                        .max_lines(1)
                        .text_overflow(TextOverflow::Ellipsis)
                }))
                .child(
                    label()
                        .text(excerpt)
                        .font_size(t::small())
                        .max_lines(2)
                        .text_overflow(TextOverflow::Ellipsis),
                ),
        )
        .into_element()
}

fn open_result(
    mut writer: Writer,
    files: State<Option<ProjectFiles>>,
    hit: &recite_writer_model::SearchHit,
) {
    let path = files
        .peek()
        .as_ref()
        .and_then(|project| project.path_for_document(&hit.document));
    let Some(path) = path else {
        return;
    };
    let _ = path;
    let location = location::Location {
        screen: location::Screen::Write,
        document: hit.document.to_string(),
        beat: Some(hit.beat.to_string()),
        passage: hit.passage.as_ref().map(ToString::to_string),
        source: false,
        view: Some(recite_config::WriterView::Script),
        ..crate::navigation::snapshot(writer)
    };
    let result = crate::navigation::route(
        writer,
        location,
        None,
        EventHandler::new(move |result: Result<(), String>| {
            if let Err(error) = result {
                writer.message.error(error);
            }
        }),
    );
    if let Err(error) = result {
        writer.message.error(error);
    }
}
