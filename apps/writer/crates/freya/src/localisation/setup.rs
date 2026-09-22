//! Language setup is transient; the manuscript keeps its place after creation.
use super::{
    Panel, create,
    messages::{MsgId, text as wording},
};
use crate::{
    design::{Button, Dialog, SearchPicker, SubmitAction, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;
use std::path::PathBuf;

pub(crate) mod languages;
mod request;
use request::{Pending, begin};

#[derive(Clone)]
pub(super) struct Setup {
    pub writer: Writer,
    pub files: State<Option<ProjectFiles>>,
}
impl PartialEq for Setup {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark && self.files == other.files
    }
}
impl Component for Setup {
    fn render(&self) -> impl IntoElement {
        let colors = t::colors();
        let writer = self.writer;
        let files = self.files;
        let mut state = writer.localisation;
        let mut message = writer.message;
        let mut locale = use_state(String::new);
        let mut choosing = use_state(|| false);
        let query = use_state(String::new);
        let choices = use_memo(move || {
            if *choosing.read() {
                std::sync::Arc::new(languages::matches(&query.read()))
            } else {
                std::sync::Arc::new(Vec::new())
            }
        });
        let mut pending = use_state(|| None::<Pending>);
        let ids: [AccessibilityId; 4] = std::array::from_fn(|_| use_a11y());
        let mut was_open = use_state(|| false);
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(50));
        use_after_side_effect(move || {
            let open = state.read().panel == Some(Panel::Create);
            if open && !*was_open.peek() {
                ids[0].request_focus();
            }
            was_open.set_if_modified(open);
        });
        if state.read().panel != Some(Panel::Create) {
            return rect().into_element();
        }
        if pending.read().is_some() && tick.elapsed() {
            tick.reset();
            let result = pending.peek().as_ref().and_then(|p| p.preparation.poll());
            if let Some(result) = result {
                let job = pending.write().take();
                if let Some(job) = job {
                    let unchanged = writer.buffers.model.peek().as_ref().is_ok_and(|m| {
                        m.document().key().as_str() == job.document
                            && m.document().source() == job.source.as_ref()
                            && !m.has_draft()
                    });
                    let result = if !unchanged || state.peek().dirty() {
                        Err(wording(MsgId::WriterCreationChanged))
                    } else {
                        result.and_then(|document| create::persist(&document, &job.path))
                    };
                    match result {
                        Ok(catalogue) => {
                            let mut current = state.write();
                            if let Err(error) = current.install(Some(catalogue)) {
                                message.error(error);
                            } else {
                                current.view = super::CatalogueView::Passage;
                                current.panel = None;
                                locale.set(String::new());
                                choosing.set(false);
                                message.info(wording(MsgId::WriterCatalogueCreated));
                                writer.inspector_focus.request_focus();
                            }
                        }
                        Err(error) => {
                            if state.peek().dirty() {
                                message.error_with_action(
                                    error,
                                    crate::messages::text(
                                        crate::messages::MsgId::WriterGuiOpenUnsavedTranslations,
                                    ),
                                    EventHandler::new(move |()| super::show_unsaved(writer)),
                                );
                            } else {
                                message.error(error);
                            }
                        }
                    }
                }
            }
        }
        let busy = pending.read().is_some();
        let root = files
            .peek()
            .as_ref()
            .map(|f| f.root().to_owned())
            .unwrap_or_else(|| PathBuf::from("."));
        let suggestion = create::language(&locale.read())
            .ok()
            .map(|locale| create::suggested_path(&root, &locale.to_string()));
        let selected = languages::caption(&locale.read()).unwrap_or_default();
        let mut content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(
                label()
                    .font_size(t::small())
                    .color(colors.muted)
                    .text(wording(if files.peek().is_some() {
                        MsgId::WriterCreateProjectScope
                    } else {
                        MsgId::WriterCreateDocumentScope
                    })),
            )
            .child(label().text(wording(MsgId::WriterTargetLanguage)))
            .child(SearchPicker {
                id: ids[0],
                input_id: ids[1],
                name: wording(MsgId::WriterChooseLanguage),
                placeholder: wording(MsgId::WriterLanguageExample),
                empty_hint: wording(MsgId::WriterLanguageExample),
                no_matches: wording(MsgId::WriterNoLanguages),
                selected,
                query,
                open: choosing,
                options: choices.read().clone(),
                enabled: !busy,
                vim: writer.preferences.read().config.ui.keymap == recite_config::Keymap::Vim,
                choose: EventHandler::new(move |option: crate::design::PickerOption| {
                    locale.set(option.value);
                }),
            })
            .child(label().font_size(t::small()).color(colors.muted).text(
                suggestion.as_ref().map_or_else(
                    || wording(MsgId::WriterCreateHelp),
                    |path| {
                        format!(
                            "{} {}",
                            wording(MsgId::WriterNewCataloguePath),
                            path.strip_prefix(&root).unwrap_or(path).display()
                        )
                    },
                ),
            ));
        if busy {
            content = content.child(label().text(wording(MsgId::WriterCreatingCatalogue)));
        }
        if !message.is_empty() {
            content = content.child(crate::feedback::NoticeView { feedback: message });
        }
        let close = EventHandler::new(move |()| {
            pending.set(None);
            state.write().panel = None;
            message.clear();
            writer.inspector_focus.request_focus();
        });
        let cancel = close.clone();
        let dismiss = EventHandler::new(move |()| {
            if *choosing.peek() {
                choosing.set(false);
                ids[0].request_focus();
            } else {
                close.call(());
            }
        });
        let enabled = !busy && !*choosing.read() && suggestion.is_some();
        let mut focus = vec![if *choosing.read() { ids[1] } else { ids[0] }, ids[2]];
        if busy {
            focus.remove(0);
        }
        if enabled {
            focus.push(ids[3]);
        }
        focus.extend(message.focus_order());
        Dialog {
            dismissal_only: false,
            title: wording(if state.peek().catalogue.is_some() {
                MsgId::WriterAddLanguage
            } else {
                MsgId::WriterStartLocalisation
            }),
            content: content.into_element(),
            close: dismiss,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
            focus_order: focus,
            actions: crate::design::actions()
                .child(
                    Button::new()
                        .a11y_id(ids[2])
                        .on_press(move |_| cancel.call(()))
                        .child(wording(MsgId::WriterCancel)),
                )
                .into_element(),
            primary: SubmitAction {
                id: ids[3],
                caption: wording(MsgId::WriterCreateCatalogue),
                enabled,
                action: EventHandler::new(move |()| {
                    let result = begin(writer, files, &locale.peek());
                    match result {
                        Ok(job) => {
                            pending.set(Some(job));
                            message.clear();
                            ids[2].request_focus();
                        }
                        Err(error) => {
                            if state.peek().dirty() {
                                message.error_with_action(
                                    error,
                                    crate::messages::text(
                                        crate::messages::MsgId::WriterGuiOpenUnsavedTranslations,
                                    ),
                                    EventHandler::new(move |()| super::show_unsaved(writer)),
                                );
                            } else {
                                message.error(error);
                            }
                        }
                    }
                }),
            },
        }
        .into_element()
    }
}
