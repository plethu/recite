use super::catalogue::Catalogue;
use super::messages::{MsgId, text as wording};
use crate::{
    design::{Button, Dialog, tokens as t},
    editing::Writer,
};
use freya::prelude::*;

pub(super) fn render(
    writer: Writer,
    files: State<Option<crate::project::ProjectFiles>>,
) -> Element {
    let mut state = writer.localisation;
    let mut message = writer.message;
    let path = use_state(String::new);
    let ids: [AccessibilityId; 10] = std::array::from_fn(|_| use_a11y());

    let save_drafts = use_a11y();
    let open_drafts = use_a11y();
    let mut focus = vec![ids[0], ids[9], ids[1], ids[3]];
    let mut was_open = use_state(|| false);
    use_after_side_effect(move || {
        let open = state.read().panel == Some(super::Panel::Files);
        if open && !*was_open.peek() {
            ids[0].request_focus();
        }
        was_open.set_if_modified(open);
    });
    if state.read().panel != Some(super::Panel::Files) {
        return rect().into_element();
    }
    let open = EventHandler::new(move |()| {
        if path.peek().trim().is_empty() {
            return;
        }
        if state.peek().dirty() {
            message.error_with_action(
                wording(MsgId::WriterOpenDrafts),
                "Open unsaved translations".into(),
                EventHandler::new(move |()| super::show_unsaved(writer)),
            );
            return;
        }
        match Catalogue::open_recoverable(std::path::Path::new(path.peek().as_str())) {
            Ok(catalogue) => {
                let mut value = state.write();
                if let Err(error) = value.install(Some(catalogue)) {
                    message.error(error);
                    return;
                }
                value.view = super::CatalogueView::Passage;
                value.panel = None;
                message.clear();
            }
            Err(error) => message.error(error),
        }
    });
    let mut content = rect()
        .spacing(t::SPACE_SM)
        .width(Size::fill())
        .child(label().text(wording(MsgId::WriterCataloguePath)))
        .child(crate::design::PathField {
            value: path,
            id: ids[0],
            browse_id: ids[9],
            kind: crate::design::PathKind::Catalogue,
            enabled: true,
            submit: open.clone(),
        })
        .child(label().text(wording(MsgId::WriterFileWorkflow)));
    if state.read().dirty() {
        focus.extend([save_drafts, open_drafts]);
        content = content
            .child(
                Button::new()
                    .a11y_id(save_drafts)
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiSaveAllTranslationDrafts,
                    ))
                    .on_press(move |_| {
                        let result = state.write().catalogue.as_mut().map(Catalogue::save_all);
                        if let Some(result) = result {
                            message.report(result, "Translations saved".into());
                        }
                    }),
            )
            .child(
                Button::new()
                    .a11y_id(open_drafts)
                    .child(crate::messages::text(
                        crate::messages::MsgId::WriterGuiOpenUnsavedTranslations,
                    ))
                    .on_press(move |_| super::show_unsaved(writer)),
            );
    }
    if let Some(catalogue) = &state.read().catalogue {
        focus.extend([ids[2], ids[4], ids[7]]);
        content = content
            .child(label().text(catalogue.path.display().to_string()))
            .child(
                Button::new()
                    .a11y_id(ids[2])
                    .on_press(move |_| {
                        let result = state.write().catalogue.as_mut().map(Catalogue::reload);
                        if let Some(result) = result {
                            message.report(result, wording(MsgId::WriterReloaded));
                        }
                    })
                    .child(wording(MsgId::WriterReload)),
            );
    }
    if state.read().catalogue.is_some() {
        content = content.child(
            Button::new()
                .a11y_id(ids[4])
                .on_press(move |_| {
                    super::comparison::prepare(writer);
                })
                .child(wording(MsgId::WriterCompare)),
        );
    }
    if state.read().catalogue.is_some() {
        content = content.child(
            Button::new()
                .a11y_id(ids[7])
                .on_press(move |_| {
                    message.clear();
                    state.write().panel = Some(super::Panel::Create);
                })
                .child(wording(MsgId::WriterAddLanguage)),
        );
    }
    if state.read().catalogue.is_some() {
        focus.push(ids[8]);
        content = content.child(
            Button::new()
                .a11y_id(ids[8])
                .on_press(move |_| {
                    message.clear();
                    match super::refresh::Preview::prepare(writer, files) {
                        Ok(preview) => {
                            let mut current = state.write();
                            current.refresh = Some(preview);
                            current.update_index = 0;
                            current.view = super::CatalogueView::Updates;
                            current.panel = None;
                            writer.inspector_focus.request_focus();
                        }
                        Err(error) => message.error(error),
                    }
                })
                .child(wording(MsgId::WriterRefresh)),
        );
    }
    if !message.is_empty() {
        content = content.child(crate::feedback::NoticeView { feedback: message });
    }
    focus.extend(message.focus_order());
    Dialog {
        dismissal_only: false,
        title: wording(MsgId::WriterPoCatalogue),
        content: content.into_element(),
        reduced_motion: writer.preferences.read().config.writer.reduced_motion,
        close: EventHandler::new(move |()| {
            state.write().panel = None;
            writer.inspector_focus.request_focus();
        }),
        focus_order: focus,
        actions: crate::design::actions()
            .child(
                Button::new()
                    .a11y_id(ids[3])
                    .on_press(move |_| {
                        state.write().panel = None;
                        writer.inspector_focus.request_focus();
                    })
                    .child(wording(MsgId::WriterClose)),
            )
            .into_element(),
        primary: crate::design::SubmitAction {
            id: ids[1],
            caption: wording(MsgId::WriterOpen),
            enabled: !path.read().trim().is_empty(),
            action: open,
        },
    }
    .into_element()
}
