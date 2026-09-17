//! Language setup is transient; the manuscript keeps its place after creation.
use super::{
    Panel, create,
    messages::{MsgId, text as wording},
};
use crate::{
    design::{Button, Dialog, tokens as t},
    editing::Writer,
    project::ProjectFiles,
};
use freya::prelude::*;
use std::path::PathBuf;

mod languages;
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
        let writer = self.writer;
        let files = self.files;
        let mut state = writer.localisation;
        let mut message = writer.message;
        let mut locale = use_state(String::new);
        let mut choosing = use_state(|| false);
        let query = use_state(String::new);
        let option_ids: [AccessibilityId; 8] = std::array::from_fn(|_| use_a11y());
        let mut pending = use_state(|| None::<Pending>);
        let ids: [AccessibilityId; 4] = std::array::from_fn(|_| use_a11y());
        let mut was_choosing = use_state(|| false);
        let mut was_open = use_state(|| false);
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(50));
        use_after_side_effect(move || {
            let open = state.read().panel == Some(Panel::Create);
            if open && !*was_open.peek() {
                ids[0].request_focus();
            }
            let picking = open && *choosing.read();
            if picking && !*was_choosing.peek() {
                ids[1].request_focus();
            }
            was_choosing.set_if_modified(picking);
            was_open.set_if_modified(open);
        });
        if state.read().panel != Some(Panel::Create) {
            return rect().into_element();
        }
        if tick.elapsed() {
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
                            current.catalogue = Some(catalogue);
                            current.panel = None;
                            locale.set(String::new());
                            choosing.set(false);
                            message.set(wording(MsgId::WriterCatalogueCreated));
                            writer.inspector_focus.request_focus();
                        }
                        Err(error) => message.set(error),
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
        let placeholder = suggestion
            .as_ref()
            .map_or_else(|| "locale/<locale>.po".into(), |p| p.display().to_string());
        let mut content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text(wording(if files.peek().is_some() {
                MsgId::WriterCreateProjectScope
            } else {
                MsgId::WriterCreateDocumentScope
            })))
            .child(
                rect()
                    .width(Size::fill())
                    .spacing(t::SPACE_XS)
                    .a11y_role(AccessibilityRole::Group)
                    .a11y_alt(wording(MsgId::WriterTargetLanguage))
                    .child(label().text(wording(MsgId::WriterTargetLanguage)))
                    .child(
                        Button::new()
                            .a11y_id(ids[0])
                            .named(languages::caption(&locale.read()).map_or_else(
                                || wording(MsgId::WriterChooseLanguage),
                                |caption| {
                                    format!("{}: {caption}", wording(MsgId::WriterChooseLanguage))
                                },
                            ))
                            .enabled(!busy)
                            .expanded(*choosing.read())
                            .width(Size::fill())
                            .on_press(move |_| {
                                let next = !*choosing.peek();
                                choosing.set(next);
                            })
                            .child(
                                rect()
                                    .horizontal()
                                    .content(Content::Flex)
                                    .width(Size::fill())
                                    .child(label().width(Size::flex(1.)).text(
                                        languages::caption(&locale.read()).unwrap_or_else(|| {
                                            wording(MsgId::WriterChooseLanguage)
                                        }),
                                    ))
                                    .child(label().text("▾")),
                            ),
                    )
                    .maybe(*choosing.read() && !busy, |r| {
                        r.child(languages::picker(
                            locale, query, choosing, ids[1], ids[0], option_ids,
                        ))
                    }),
            )
            .child(label().text(wording(MsgId::WriterNewCataloguePath)))
            .child(label().text(placeholder))
            .child(label().text(wording(MsgId::WriterCreateHelp)));
        if busy {
            content = content.child(label().text(wording(MsgId::WriterCreatingCatalogue)));
        }
        if !message.read().is_empty() {
            content = content.child(label().text(message.read().clone()));
        }
        let close = EventHandler::new(move |()| {
            pending.set(None);
            state.write().panel = None;
            message.set(String::new());
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
        let focus = if busy {
            vec![ids[2]]
        } else {
            let mut order = vec![ids[0]];
            if *choosing.read() {
                order.push(ids[1]);
                order.extend(
                    option_ids
                        .into_iter()
                        .take(languages::matches(&query.read()).len()),
                );
            }
            order.push(ids[2]);
            if !*choosing.read() {
                order.push(ids[3]);
            }
            order
        };
        Dialog {
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
                .child(
                    Button::new()
                        .filled()
                        .a11y_id(ids[3])
                        .enabled(!busy && !*choosing.read())
                        .on_press(move |_| {
                            let result = begin(writer, files, &locale.peek());
                            match result {
                                Ok(job) => {
                                    pending.set(Some(job));
                                    message.set(String::new());
                                    ids[2].request_focus();
                                }
                                Err(error) => message.set(error),
                            }
                        })
                        .child(wording(MsgId::WriterCreateCatalogue)),
                )
                .into_element(),
        }
        .into_element()
    }
}
