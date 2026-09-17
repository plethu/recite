use super::catalogue::Catalogue;
use super::messages::{MsgId, text as wording};
use crate::{
    design::{Button, Dialog, tokens as t},
    editing::Writer,
};
use freya::prelude::*;

pub(super) fn render(writer: Writer) -> Element {
    let mut state = writer.localisation;
    let mut message = writer.message;
    let path = use_state(String::new);
    let ids: [AccessibilityId; 8] = std::array::from_fn(|_| use_a11y());
    let mut comparison = use_state(|| None::<super::catalogue::Comparison>);
    let mut focus = vec![ids[0], ids[1], ids[3]];
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
    let mut content = rect()
        .spacing(t::SPACE_SM)
        .width(Size::fill())
        .child(label().text(wording(MsgId::WriterCataloguePath)))
        .child(
            Input::new(path)
                .placeholder("/path/to/fr.po")
                .a11y_id(ids[0])
                .width(Size::fill())
                .on_pre_key_down(crate::closing::text_input_key),
        )
        .child(label().text(wording(MsgId::WriterFileWorkflow)));
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
                            message.set(
                                result
                                    .err()
                                    .unwrap_or_else(|| wording(MsgId::WriterReloaded)),
                            );
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
                    if let Some(catalogue) = &state.peek().catalogue {
                        match catalogue.compare() {
                            Ok(text) => comparison.set(Some(text)),
                            Err(error) => message.set(error),
                        }
                    }
                })
                .child(wording(MsgId::WriterCompare)),
        );
    }
    if state.read().catalogue.is_some() {
        content = content.child(
            Button::new()
                .a11y_id(ids[7])
                .on_press(move |_| {
                    message.set(String::new());
                    state.write().panel = Some(super::Panel::Create);
                })
                .child(wording(MsgId::WriterAddLanguage)),
        );
    }
    if let Some(text) = comparison.read().as_ref() {
        focus.extend([ids[5], ids[6]]);
        content = content
            .child(label().text(wording(MsgId::WriterCompareHelp)))
            .child(label().text(text.text.clone()));
        for (id, keep, caption) in [
            (ids[5], true, MsgId::WriterKeepDrafts),
            (ids[6], false, MsgId::WriterUseFile),
        ] {
            content = content.child(
                Button::new()
                    .a11y_id(id)
                    .on_press(move |_| {
                        if let Some(catalogue) = state.write().catalogue.as_mut() {
                            let expected = comparison
                                .peek()
                                .as_ref()
                                .map(|c| c.fingerprint.clone())
                                .ok_or_else(|| wording(MsgId::WriterCompare))
                                .and_then(|expected| catalogue.accept_external(keep, &expected));
                            match expected {
                                Ok(()) => {
                                    comparison.set(None);
                                    message.set(wording(MsgId::WriterCompared));
                                }
                                Err(error) => message.set(error),
                            }
                        }
                    })
                    .child(wording(caption)),
            );
        }
    }
    if !message.read().is_empty() {
        content = content.child(label().text(message.read().clone()));
    }
    Dialog {
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
            .child(
                Button::new()
                    .a11y_id(ids[1])
                    .filled()
                    .on_press(move |_| {
                        if state.peek().dirty() {
                            message.set(wording(MsgId::WriterOpenDrafts));
                            return;
                        }
                        match Catalogue::open(std::path::Path::new(path.peek().as_str())) {
                            Ok(catalogue) => {
                                let mut value = state.write();
                                value.catalogue = Some(catalogue);
                                value.panel = None;
                                message.set(String::new());
                            }
                            Err(error) => message.set(error),
                        }
                    })
                    .child(wording(MsgId::WriterOpen)),
            )
            .into_element(),
    }
    .into_element()
}
