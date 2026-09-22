//! Catalogue-wide work replaces the scene drawer; passage review remains separate.
use super::Preview;
use crate::localisation::{
    CatalogueView,
    messages::{MsgId, text as wording},
};
use crate::{
    design::{Button, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;

#[derive(Clone)]
pub(crate) struct RefreshScreen {
    pub writer: Writer,
    pub files: State<Option<ProjectFiles>>,
}
impl PartialEq for RefreshScreen {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.files == other.files
    }
}
impl Component for RefreshScreen {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let files = self.files;
        let mut state = writer.localisation;
        let mut scope = use_state(|| false);
        let current = state.read();
        let mut body = rect()
            .width(Size::fill())
            .height(Size::fill())
            .content(Content::Flex)
            .spacing(t::SPACE_MD)
            .padding(t::SPACE_LG)
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_SM)
                    .child(
                        label()
                            .text(wording(MsgId::WriterSourceUpdates))
                            .font_size(t::title()),
                    )
                    .child(
                        Button::new()
                            .flat()
                            .on_press(move |_| state.write().view = CatalogueView::Passage)
                            .child(wording(MsgId::WriterReadPassage)),
                    ),
            );
        let mut identity = rect()
            .horizontal()
            .content(Content::Flex)
            .width(Size::fill())
            .spacing(t::SPACE_SM);
        if let Some(catalogue) = &current.catalogue {
            let root = files.peek();
            let path = root
                .as_ref()
                .and_then(|p| catalogue.path.strip_prefix(p.root()).ok())
                .unwrap_or(&catalogue.path);
            let language = catalogue
                .document
                .headers()
                .iter()
                .find(|h| h.key().eq_ignore_ascii_case("Language"))
                .map(|h| h.value());
            let caption = language.map_or_else(
                || path.display().to_string(),
                |language| format!("{language} · {}", path.display()),
            );
            identity = identity.child(
                label()
                    .width(Size::flex(1.))
                    .text(caption)
                    .font_size(t::small()),
            );
        }
        body = body.child(
            identity.child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        let next = !*scope.peek();
                        scope.set(next);
                    })
                    .child(wording(MsgId::WriterRefreshScope)),
            ),
        );
        if *scope.read() {
            body = body.child(label().text(wording(MsgId::WriterRefreshScopeHelp)));
        }
        let mut actions = rect().horizontal().spacing(t::SPACE_SM).child(
            Button::new()
                .on_press(move |_| recheck(writer, files))
                .child(wording(MsgId::WriterRefreshRecheck)),
        );
        if let Some(preview) = &current.refresh {
            body = body.child(label().text(preview.summary.clone()));
            if preview.applied {
                body = body.child(label().text(wording(MsgId::WriterRefreshDone)));
            }
            let index = current
                .update_index
                .min(preview.changes.len().saturating_sub(1));
            let page = index / 24;
            let mut list = rect().width(Size::px(230.)).spacing(t::SPACE_XS);
            for (i, change) in preview.changes.iter().enumerate().skip(page * 24).take(24) {
                let excerpt = change
                    .new
                    .as_ref()
                    .or(change.old.as_ref())
                    .map_or(String::new(), |s| s.chars().take(100).collect());
                list = list.child(
                    Button::new()
                        .flat()
                        .selected(index == i)
                        .width(Size::fill())
                        .on_press(move |_| state.write().update_index = i)
                        .child(
                            rect()
                                .width(Size::fill())
                                .child(label().text(excerpt))
                                .child(
                                    label()
                                        .text(format!(
                                            "{} · {}",
                                            wording(change.kind.label()),
                                            change.caption
                                        ))
                                        .font_size(t::small()),
                                ),
                        ),
                );
            }
            if preview.changes.len() > 24 {
                let last = preview.changes.len() - 1;
                list = list.child(
                    rect()
                        .horizontal()
                        .spacing(t::SPACE_SM)
                        .child(
                            Button::new()
                                .enabled(page > 0)
                                .on_press(move |_| {
                                    state.write().update_index = (page.saturating_sub(1)) * 24
                                })
                                .child(wording(MsgId::WriterPrevious)),
                        )
                        .child(
                            Button::new()
                                .enabled((page + 1) * 24 <= last)
                                .on_press(move |_| {
                                    state.write().update_index = ((page + 1) * 24).min(last)
                                })
                                .child(wording(MsgId::WriterNext)),
                        ),
                );
            }
            if let Some(change) = preview.changes.get(index) {
                body = body.child(
                    rect()
                        .horizontal()
                        .content(Content::Flex)
                        .width(Size::fill())
                        .spacing(t::SPACE_XL)
                        .height(Size::flex(1.))
                        .child(
                            ScrollView::new()
                                .width(Size::px(230.))
                                .height(Size::fill())
                                .child(list),
                        )
                        .child(
                            ScrollView::new()
                                .width(Size::flex(1.))
                                .height(Size::fill())
                                .child(super::detail::render(change, writer)),
                        ),
                );
            } else {
                body = body.child(label().text(wording(MsgId::WriterRefreshNoChanges)));
            }
            actions = actions.child(
                Button::new()
                    .filled()
                    .enabled(!preview.applied)
                    .on_press(move |_| save(writer, files))
                    .child(wording(MsgId::WriterRefreshSave)),
            );
            if preview.applied {
                actions = actions.child(
                    Button::new()
                        .on_press(move |_| {
                            let mut attention = writer.queue.attention;
                            attention.set(true);
                            let mut page = writer.queue.page;
                            page.set(0);
                            state.write().view = CatalogueView::Queue;
                        })
                        .child(wording(MsgId::WriterRefreshReview)),
                );
            }
        }
        body = body.child(actions);
        if !writer.message.is_empty() {
            body = body.child(crate::feedback::NoticeView {
                feedback: writer.message,
            });
        }
        body
    }
}
fn recheck(mut writer: Writer, files: State<Option<ProjectFiles>>) {
    writer.message.clear();
    match Preview::prepare(writer, files) {
        Ok(preview) => {
            let mut current = writer.localisation.write();
            current.refresh = Some(preview);
            current.update_index = 0;
        }
        Err(error) => writer.message.error(error),
    }
}
fn save(mut writer: Writer, files: State<Option<ProjectFiles>>) {
    let preview = { writer.localisation.write().refresh.take() };
    let Some(mut preview) = preview else {
        return;
    };
    writer.message.clear();
    match preview.apply(writer, files) {
        Ok(()) => {
            preview.applied = true;
            writer.message.info(wording(MsgId::WriterRefreshed));
        }
        Err(error) => writer.message.error(error),
    }
    writer.localisation.write().refresh = Some(preview);
}
