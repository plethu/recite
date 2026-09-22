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
                            if !e.modifiers.is_empty() {
                                return;
                            }
                            let next = match &e.key {
                                Key::Named(NamedKey::ArrowRight) => {
                                    Some((index + 1) % targets.len())
                                }
                                Key::Named(NamedKey::ArrowLeft) => {
                                    Some((index + targets.len() - 1) % targets.len())
                                }
                                Key::Named(NamedKey::Home) => Some(0),
                                Key::Named(NamedKey::End) => Some(targets.len() - 1),
                                Key::Character(key) if vim && key == "j" => {
                                    Some((index + 1) % targets.len())
                                }
                                Key::Character(key) if vim && key == "k" => {
                                    Some((index + targets.len() - 1) % targets.len())
                                }
                                _ => None,
                            };
                            if let Some(next) = next {
                                e.stop_propagation();
                                e.prevent_default();
                                let (path, id) = &targets[next];
                                match writer.open_document(path) {
                                    Ok(()) => {
                                        id.request_focus();
                                    }
                                    Err(e) => writer.message.error(e),
                                }
                            }
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
                                .on_press(move |_| match writer.open_document(&target) {
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
                                    // Harvest before checking: rendered dirty state can lag typing.
                                    writer.buffers.harvest();
                                    let state = writer.buffers.model.peek();
                                    let dirty = state
                                        .as_ref()
                                        .ok()
                                        .zip(writer.files.peek().as_ref())
                                        .is_none_or(|(model, files)| {
                                            files
                                                .open_documents(model)
                                                .iter()
                                                .any(|(p, d)| p == &close_target && *d)
                                        });
                                    if dirty {
                                        closing.set(Some(close_target.clone()));
                                        submit_id.request_focus();
                                    } else {
                                        close(writer, &close_target);
                                    }
                                })
                                .child("×")
                        })),
                );
            }
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
                    enabled: true,
                    action: EventHandler::new(move |()| {
                        let result = writer
                            .buffers
                            .switch(writer.files, &path, writer.dark, |_| Ok(()))
                            .and_then(|()| writer.buffers.save(writer.files));
                        match result {
                            Ok(()) => {
                                if close(writer, &path) {
                                    closing.set(None);
                                }
                            }
                            Err(e) => writer.message.error(e),
                        }
                    }),
                },
                focus_order: vec![cancel_id, submit_id],
                close: close_dialog,
                reduced_motion: writer.preferences.read().config.writer.reduced_motion,
            }
        });
        rect()
            .width(Size::fill())
            .child(
                ScrollView::new()
                    .direction(Direction::Horizontal)
                    .height(Size::auto())
                    .width(Size::fill())
                    .child(tabs),
            )
            .maybe_child(dialog)
    }
}
fn close(mut writer: Writer, path: &std::path::Path) -> bool {
    let result = {
        let mut state = writer.buffers.model.write();
        let mut files = writer.files.write();
        match (state.as_mut(), files.as_mut()) {
            (Ok(model), Some(files)) => {
                files.close_document(model, path).map_err(|e| e.to_string())
            }
            _ => return false,
        }
    };
    if let Err(e) = result {
        writer.message.error(e);
        return false;
    }
    let model = writer.buffers.model.peek();
    if let Ok(model) = model.as_ref() {
        writer.buffers.editor.set(crate::editing::editor_data(
            model.draft(),
            model.view() == &recite_writer_model::View::Source,
            writer.dark,
        ));
        writer.buffers.prose.set(model.draft().into());
    }
    writer.selection.set(None);
    true
}
