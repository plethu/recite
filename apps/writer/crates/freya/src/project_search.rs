//! Bounded project search results retain document and beat context.
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;

#[derive(Clone)]
struct SearchSnapshot(Option<std::sync::Arc<recite_writer_model::SearchIndex>>);
impl PartialEq for SearchSnapshot {
    fn eq(&self, other: &Self) -> bool {
        match (&self.0, &other.0) {
            (Some(a), Some(b)) => std::sync::Arc::ptr_eq(a, b),
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
        crate::design::use_list_reveal(Some(query), active, scroll, 84., 1);
        let writer = self.writer;
        let files = self.files;
        let index =
            use_memo(move || SearchSnapshot(files.read().as_ref().map(ProjectFiles::search_index)));
        let results = use_memo(move || {
            index
                .read()
                .0
                .as_ref()
                .map(|index| index.search(&query.read(), *limit.read()))
                .unwrap_or_default()
        });
        let (total, hits) = &*results.read();
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
                        let hit = results.peek().1.get(index).cloned();
                        if let Some(hit) = hit {
                            open_result(writer, files, &hit);
                        }
                    }),
                });
        if !query.read().trim().is_empty() {
            let list = VirtualScrollView::new_with_data(
                (hits.clone(), *active.read()),
                move |item, (hits, active)| {
                    rect()
                        .key(item.index)
                        .width(Size::fill())
                        .height(Size::px(84.))
                        .overflow(Overflow::Clip)
                        .child(result_row(
                            writer,
                            files,
                            hits[item.index].clone(),
                            *active == Some(item.index),
                        ))
                        .into_element()
                },
            )
            .scroll_controller(scroll)
            .length(hits.len())
            .item_size(84.)
            .height(Size::px(200.))
            .width(Size::fill());
            content = content
                .child(
                    label()
                        .text(format!("{} of {total} saved passages", hits.len()))
                        .font_size(t::small()),
                )
                .child(list);
            if hits.is_empty() {
                content = content.child(label().text(crate::messages::text(
                    crate::messages::MsgId::WriterGuiNoMatchingSavedPassagesTryFewerWords,
                )));
            }
            if hits.len() < *total {
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

fn result_row(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
    hit: recite_writer_model::SearchHit,
    active: bool,
) -> Element {
    let name = format!("{} · {} · {}", hit.document, hit.beat, hit.text);
    let caption = format!(
        "{} · {}",
        hit.document,
        crate::palette::display_name(&hit.beat)
    );
    let excerpt = hit.text.chars().take(90).collect::<String>();
    Button::new()
        .flat()
        .width(Size::fill())
        .selected(active)
        .named(name)
        .on_press(move |_| open_result(writer, files, &hit))
        .child(
            rect()
                .width(Size::fill())
                .child(label().text(caption).font_size(t::small()))
                .child(label().text(excerpt).font_size(t::small())),
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
    let same = files.peek().as_ref().is_some_and(|p| p.current == path);
    if !same
        && let Err(error) = writer.buffers.switch(files, &path, writer.dark, |m| {
            if m.view() == &recite_writer_model::View::Block(hit.beat.clone()) {
                Ok(())
            } else {
                m.inspect_block(&hit.beat)
            }
        })
    {
        writer.message.error(error);
        return;
    }
    writer.inspect(&hit.beat);
}
