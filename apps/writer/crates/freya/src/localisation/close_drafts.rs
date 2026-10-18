//! Resolve unsaved PO work at the close boundary without hiding its location.
use super::{CatalogueView, navigation::Destination};
use crate::{
    design::{Button, Dialog, SubmitAction, tokens as t},
    editing::Writer,
};
use freya::prelude::*;
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct CloseDrafts {
    pub writer: Writer,
    pub cancel: EventHandler<()>,
    pub resolved: EventHandler<()>,
}
impl PartialEq for CloseDrafts {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
            && self.cancel == other.cancel
            && self.resolved == other.resolved
    }
}
impl Component for CloseDrafts {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut state = writer.localisation;
        let cancel = self.cancel.clone();
        let resolved = self.resolved.clone();
        let mut failure = use_state(|| None::<String>);
        let mut row_ids = use_state(BTreeMap::new);
        let keep = use_a11y();
        let discard = use_a11y();
        let save = use_a11y();
        use_after_side_effect(move || keep.request_focus());
        let current = state.read();
        let Some(catalogue) = &current.catalogue else {
            return rect().into_element();
        };
        let passages = writer
            .buffers
            .model
            .read()
            .as_ref()
            .ok()
            .and_then(|m| m.document().cached_passages())
            .unwrap_or_default();
        let entries: Vec<_> = catalogue
            .document
            .entries()
            .iter()
            .filter(|e| catalogue.changed(e.id()))
            .collect();
        let saving = catalogue.saving;
        let count = entries.len();
        let drafts = if count == 1 { "draft" } else { "drafts" };
        let first = entries.first().and_then(|e| e.context()).map(str::to_owned);
        let language = catalogue
            .document
            .headers()
            .iter()
            .find(|h| h.key() == "Language")
            .map(|h| h.value())
            .unwrap_or("Translation");
        let path = catalogue.path.display().to_string();
        let mut content = rect()
            .width(Size::fill())
            .spacing(t::SPACE_SM)
            .child(label().text(format!("{count} unsaved translation {drafts} · {language}")))
            .child(label().text(path.clone()).font_size(t::small()));
        if !writer.message.is_empty() {
            content = content.child(crate::feedback::NoticeView {
                feedback: writer.message,
            });
        }
        if saving {
            content = content
                .child(label().text(crate::messages::text(crate::messages::MsgId::WriterSaving)));
        }
        if let Some(error) = failure.read().as_ref() {
            content = content.child(label().text(error.clone()));
        }
        let mut order = vec![keep];
        for entry in entries {
            let id = *row_ids
                .write()
                .entry(entry.id())
                .or_insert_with(AccessibilityId::new_unique);
            order.push(id);
            let context = entry.context().map(str::to_owned);
            let caption = Destination::resolve(entry, &passages)
                .map(|d| d.caption())
                .unwrap_or_else(|| "Catalogue entry".into());
            let draft = catalogue
                .draft(entry.id())
                .map(|d| d.forms.join(" · "))
                .unwrap_or_default();
            let dismiss = cancel.clone();
            content = content.child(
                Button::new()
                    .flat()
                    .width(Size::fill())
                    .a11y_id(id)
                    .on_press(move |_| {
                        dismiss.call(());
                        open(state, context.clone());
                    })
                    .child(
                        rect()
                            .width(Size::fill())
                            .spacing(t::SPACE_XS)
                            .child(label().text(format!("Open {caption}")))
                            .child(
                                label()
                                    .width(Size::fill())
                                    .text(entry.source_text().chars().take(180).collect::<String>())
                                    .font_size(t::small()),
                            )
                            .child(label().width(Size::fill()).text(format!(
                                "Draft: {}",
                                if draft.is_empty() { "(empty)" } else { &draft }
                            ))),
                    ),
            );
        }
        order.extend([discard, save]);
        drop(current);
        let dismiss = cancel.clone();
        let discard_resolved = resolved.clone();
        Dialog {
            title: "Unsaved translations".into(),
            content: content.into_element(),
            actions: crate::design::actions()
                .child(
                    Button::new()
                        .a11y_id(keep)
                        .child(crate::messages::text(
                            crate::messages::MsgId::WriterGuiKeepEditing,
                        ))
                        .on_press(move |_| {
                            dismiss.call(());
                            open(state, first.clone());
                        }),
                )
                .child(
                    Button::new()
                        .a11y_id(discard)
                        .enabled(!saving)
                        .child(format!("Discard {count} {drafts} and quit"))
                        .on_press(move |_| {
                            if let Some(catalogue) = state.write().catalogue.as_mut() {
                                let ids: Vec<_> = catalogue
                                    .document
                                    .entries()
                                    .iter()
                                    .filter(|e| catalogue.changed(e.id()))
                                    .map(|e| e.id())
                                    .collect();
                                for id in ids {
                                    catalogue.discard(id);
                                }
                                if let Err(error) = catalogue.flush_recovery() {
                                    failure.set(Some(error));
                                    return;
                                }
                            }
                            discard_resolved.call(());
                        }),
                )
                .into_element(),
            primary: SubmitAction {
                id: save,
                caption: "Save all and quit".into(),
                enabled: !saving,
                action: EventHandler::new(move |()| {
                    let resolved=resolved.clone();
                    crate::editor_work::translations(writer,None,EventHandler::new(move |result:Result<(),String>| {
                        match result {
                            Ok(()) if !writer.localisation.peek().dirty()=>resolved.call(()),
                            Ok(())=> { let mut message=writer.message; message.info(crate::messages::text(crate::messages::MsgId::WriterNewerEditsRemain)); },
                            Err(error)=> { let mut message=writer.message; message.error(format!("Could not save translations. {error}. Your drafts are still available.")); },
                        }
                    }));
                }),
            },
            focus_order: order,
            close: cancel,
            reduced_motion: writer.preferences.read().config.writer.reduced_motion,
            dismissal_only: false,
        }
        .into_element()
    }
}
fn open(mut state: State<super::Localisation>, context: Option<String>) {
    let mut current = state.write();
    current.active = true;
    current.view = if context.is_some() {
        CatalogueView::Entry
    } else {
        CatalogueView::Queue
    };
    current.entry_context = context;
}
