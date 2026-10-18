//! Tabs expose retained sessions; closing never implicitly discards a draft.
use crate::{
    design::{Button, Dialog, SubmitAction, tokens as t},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(crate) struct DocumentTabs {
    pub writer: Writer,
}
impl PartialEq for DocumentTabs {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for DocumentTabs {
    fn render(&self) -> impl IntoElement {
        let mut writer = self.writer;
        let mut closing = use_state(|| None::<PathBuf>);
        let ids = use_hook(|| {
            std::rc::Rc::new(std::cell::RefCell::new(std::collections::BTreeMap::<
                PathBuf,
                AccessibilityId,
            >::new()))
        });
        let submit_id = use_a11y();
        let cancel_id = use_a11y();
        let opened = writer.files.read();
        let model = writer.buffers.model.read();
        let mut tabs = rect()
            .horizontal()
            .spacing(t::SPACE_XS)
            .a11y_role(AccessibilityRole::TabList)
            .a11y_alt(text(MsgId::WriterOpenDocuments));
        if let (Some(project), Ok(model)) = (opened.as_ref(), model.as_ref()) {
            let documents = project.open_documents(model);
            {
                let mut ids = ids.borrow_mut();
                ids.retain(|path, _| documents.iter().any(|(p, _)| p == path));
                for (path, _) in &documents {
                    ids.entry(path.clone())
                        .or_insert_with(AccessibilityId::new_unique);
                }
            }
            let vim = writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim;
            for (index, (path, dirty)) in documents.iter().enumerate() {
                let id = ids.borrow()[path];
                let targets: Vec<_> = documents
                    .iter()
                    .map(|(p, _)| (p.clone(), ids.borrow()[p]))
                    .collect();
                let target = path.clone();
                let close_target = path.clone();
                let caption = path
                    .strip_prefix(project.root())
                    .unwrap_or(path)
                    .display()
                    .to_string();
                let active = path == &project.current;
                let dirty = *dirty;
                tabs = tabs.child(
                    rect()
                        .horizontal()
                        .cross_align(Alignment::Center)
                        .on_key_down(move |e: Event<KeyboardEventData>| {
                            navigate_tab(writer, e, &targets, index, vim);
                        })
                        .child(
                            Button::new()
                                .tab(active)
                                .a11y_id(id)
                                .named(format!(
                                    "{caption}{}",
                                    if dirty {
                                        format!(" · {}", text(MsgId::WriterUnsaved))
                                    } else {
                                        String::new()
                                    }
                                ))
                                .on_press(move |_| match writer.open_document(&target, None) {
                                    Ok(()) => {}
                                    Err(e) => writer.message.error(e),
                                })
                                .child(format!("{caption}{}", if dirty { " •" } else { "" })),
                        )
                        .maybe_child((documents.len() > 1).then(|| {
                            Button::new()
                                .flat()
                                .named(format!("{} {caption}", text(MsgId::WriterClose)))
                                .on_press(move |_| {
                                    request_close(writer, &close_target, closing, submit_id);
                                })
                                .child("×")
                        })),
                );
            }
        }
        if let (None, Ok(model)) = (opened.as_ref(), model.as_ref()) {
            // Without a project the band still names the open file, as one passive tab.
            let key = model.document().key().as_str();
            let caption = if key.ends_with(".recite") {
                key.to_owned()
            } else {
                format!("{key}.recite")
            };
            tabs = tabs.child(
                rect()
                    .height(Size::px(t::control_height()))
                    .padding((0., t::SPACE_SM))
                    .main_align(Alignment::Center)
                    .font_weight(FontWeight::MEDIUM)
                    .border(
                        Border::new()
                            .width(BorderWidth {
                                bottom: t::FOCUS_WIDTH,
                                ..Default::default()
                            })
                            .alignment(BorderAlignment::Inner)
                            .fill(t::colors().teal),
                    )
                    .a11y_role(AccessibilityRole::Tab)
                    .a11y_builder(|node| node.set_selected(true))
                    .child(label().text(caption)),
            );
        }
        drop(model);
        drop(opened);
        let dialog = closing.read().clone().map(|path| {
            let close_dialog = EventHandler::new(move |()| {
                closing.set(None);
                writer.inspector_focus.request_focus();
            });
            let cancel = close_dialog.clone();
            Dialog {
                dismissal_only: false,
                title: text(MsgId::WriterSaveCloseDocument),
                content: label()
                    .text(text(MsgId::WriterCloseDocumentDraft))
                    .into_element(),
                actions: Button::new()
                    .a11y_id(cancel_id)
                    .on_press(move |_| cancel.call(()))
                    .child(text(MsgId::WriterCancel))
                    .into_element(),
                primary: SubmitAction {
                    id: submit_id,
                    caption: text(MsgId::WriterSaveCloseDocument),
                    enabled: !writer.files.read().as_ref().is_some_and(|p| p.saving),
                    action: EventHandler::new(move |()| {
                        let saved_path = path.clone();
                        crate::editor_work::start(
                            writer,
                            crate::editor_work::SaveScope::Document(path.clone()),
                            EventHandler::new(move |result| match result {
                                Ok(()) => {
                                    if closing
                                        .try_peek()
                                        .is_none_or(|c| c.as_ref() != Some(&saved_path))
                                    {
                                        return;
                                    }
                                    close(writer, &saved_path, closing);
                                }
                                Err(error) => writer.message.error(error),
                            }),
                        );
                    }),
                },
                focus_order: vec![cancel_id, submit_id],
                close: close_dialog,
                reduced_motion: writer.preferences.read().config.writer.reduced_motion,
            }
        });
        // The row is reserved even without documents, so opening a project never moves the toolbar.
        rect()
            .width(Size::fill())
            .height(Size::px(t::control_height() + t::SPACE_XS))
            .padding((t::SPACE_XS, t::SPACE_SM, 0., t::SPACE_SM))
            .border(
                Border::new()
                    .width(BorderWidth {
                        bottom: 1.,
                        ..Default::default()
                    })
                    .fill(t::colors().rule),
            )
            .child(
                ScrollView::new()
                    .direction(Direction::Horizontal)
                    .height(Size::fill())
                    .width(Size::fill())
                    .child(tabs),
            )
            .maybe_child(dialog)
    }
}

fn navigate_tab(
    mut writer: Writer,
    event: Event<KeyboardEventData>,
    targets: &[(PathBuf, AccessibilityId)],
    index: usize,
    vim: bool,
) {
    if !event.modifiers.is_empty() {
        return;
    }
    let next = match &event.key {
        Key::Named(NamedKey::ArrowRight) => Some((index + 1) % targets.len()),
        Key::Named(NamedKey::ArrowLeft) => Some((index + targets.len() - 1) % targets.len()),
        Key::Named(NamedKey::Home) => Some(0),
        Key::Named(NamedKey::End) => Some(targets.len() - 1),
        Key::Character(key) if vim && key == "j" => Some((index + 1) % targets.len()),
        Key::Character(key) if vim && key == "k" => {
            Some((index + targets.len() - 1) % targets.len())
        }
        _ => None,
    };
    let Some(next) = next else {
        return;
    };
    event.stop_propagation();
    event.prevent_default();
    let (path, id) = &targets[next];
    match writer.open_document(path, Some(*id)) {
        Ok(()) => {}
        Err(error) => writer.message.error(error),
    }
}

fn request_close(
    writer: Writer,
    path: &std::path::Path,
    mut closing: State<Option<PathBuf>>,
    submit_id: AccessibilityId,
) {
    if writer.files.peek().as_ref().is_some_and(|p| p.saving) {
        return;
    }
    // Harvest before checking: rendered dirty state can lag typing.
    writer.buffers.harvest();
    let dirty = {
        let state = writer.buffers.model.peek();
        state
            .as_ref()
            .ok()
            .zip(writer.files.peek().as_ref())
            .is_none_or(|(model, files)| {
                files
                    .open_documents(model)
                    .iter()
                    .any(|(candidate, dirty)| candidate == path && *dirty)
            })
    };
    if dirty {
        closing.set(Some(path.to_owned()));
        submit_id.request_focus();
    } else {
        close(writer, path, closing);
    }
}

fn close(mut writer: Writer, path: &std::path::Path, mut closing: State<Option<PathBuf>>) {
    writer.buffers.harvest();
    let result = (|| {
        let state = writer.buffers.model.peek();
        let files = writer.files.peek();
        let (Ok(model), Some(files)) = (state.as_ref(), files.as_ref()) else {
            return Err("Project is no longer open.".to_string());
        };
        files
            .require_saved_document(model, path)
            .map_err(|e| e.to_string())?;
        if files.current != path {
            return Ok(None);
        }
        files
            .open_documents(model)
            .into_iter()
            .find(|(p, _)| p != path)
            .map(|(next, _)| Some(next))
            .ok_or_else(|| "Keep at least one document open.".to_string())
    })();
    let next = match result {
        Ok(next) => next,
        Err(error) => {
            writer.message.error(error);
            return;
        }
    };
    let expected = closing.peek().clone();
    let path = path.to_owned();
    let complete = EventHandler::new(move |result: Result<(), String>| {
        if closing.try_peek().is_none_or(|c| *c != expected) {
            return;
        }
        let result = result.and_then(|()| {
            let mut state = writer.buffers.model.write();
            let mut files = writer.files.write();
            let (Ok(model), Some(files)) = (state.as_mut(), files.as_mut()) else {
                return Err("Project is no longer open.".to_string());
            };
            if files.current == path {
                return Err("The document is still active; retry closing.".into());
            }
            files
                .close_document(model, &path)
                .map_err(|e| e.to_string())
        });
        match result {
            Ok(()) => {
                writer.close_scene(&path);
                closing.set(None);
            }
            Err(error) => writer.message.error(error),
        }
    });
    if let Some(next) = next {
        if let Err(error) = writer.open_document_then(&next, complete) {
            writer.message.error(error);
        }
    } else {
        complete.call(Ok(()));
    }
}
