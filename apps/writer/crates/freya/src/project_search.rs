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
        let writer = self.writer;
        let files = self.files;
        let index =
            use_memo(move || SearchSnapshot(files.read().as_ref().map(ProjectFiles::search_index)));
        let results = use_memo(move || {
            index
                .read()
                .0
                .as_ref()
                .map(|index| index.search(&query.read(), 100))
                .unwrap_or_default()
        });
        let (total, hits) = &*results.read();
        let mut content = rect().width(Size::fill()).spacing(t::SPACE_XS).child(
            Input::new(query)
                .width(Size::fill())
                .placeholder("Search project words…")
                .on_pre_key_down(crate::closing::text_input_key),
        );
        if !query.read().trim().is_empty() {
            let list = VirtualScrollView::new_with_data(hits.clone(), move |item, hits| {
                rect()
                    .key(item.index)
                    .width(Size::fill())
                    .height(Size::px(84.))
                    .overflow(Overflow::Clip)
                    .child(result_row(writer, files, hits[item.index].clone()))
                    .into_element()
            })
            .length(hits.len())
            .item_size(84.)
            .height(Size::px(200.))
            .width(Size::fill());
            content = content
                .child(
                    label()
                        .text(format!("{} of {total} saved passages", hits.len()))
                        .font_size(t::TEXT_SMALL),
                )
                .child(list);
        }
        content
    }
}

fn result_row(
    writer: Writer,
    files: State<Option<ProjectFiles>>,
    hit: recite_writer_model::SearchHit,
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
        .named(name)
        .on_press(move |_| open_result(writer, files, &hit))
        .child(
            rect()
                .width(Size::fill())
                .child(label().text(caption).font_size(t::TEXT_SMALL))
                .child(label().text(excerpt).font_size(t::TEXT_SMALL)),
        )
        .into_element()
}

fn open_result(
    mut writer: Writer,
    mut files: State<Option<ProjectFiles>>,
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
    if !same {
        if !writer.buffers.can_leave(files.peek().as_ref()) {
            writer
                .message
                .set("Save changes before opening a result in another scene.".into());
            return;
        }
        if let Some(project) = files.write().as_mut() {
            match project.select(&path) {
                Ok(next) => writer.buffers.install(next, writer.dark),
                Err(error) => {
                    writer.message.set(error.to_string());
                    return;
                }
            }
        }
    }
    writer.inspect(&hit.beat);
}
