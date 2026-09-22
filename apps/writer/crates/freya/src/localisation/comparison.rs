//! External-file decisions stay bound to the fingerprint the user inspected.
use super::{
    CatalogueView,
    messages::{MsgId, text},
};
use crate::{
    design::{Button, ComparisonView, tokens as t},
    editing::Writer,
};
use freya::prelude::*;

#[derive(Clone)]
pub(super) struct ExternalComparison {
    pub writer: Writer,
}
impl PartialEq for ExternalComparison {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for ExternalComparison {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let mut state = writer.localisation;
        let mut message = writer.message;
        let action_id = use_a11y();
        let mut choice = use_state(|| None::<bool>);
        let current = state.read();
        let mut body = rect()
            .width(Size::fill())
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .child(
                label()
                    .text(text(MsgId::WriterCompare))
                    .font_size(t::title()),
            )
            .child(
                Button::new()
                    .flat()
                    .on_press(move |_| {
                        let previous = state.peek().comparison_return;
                        state.write().view = previous;
                    })
                    .child(text(MsgId::WriterReadPassage)),
            );
        if let Some(comparison) = &current.comparison {
            let expected = comparison.fingerprint.clone();
            body = body.child(ComparisonView {
                code: false,
                before: text(MsgId::WriterDraftVersion),
                after: text(MsgId::WriterDiskVersion),
                rows: comparison.rows.clone(),
            });
            let mut choices = crate::design::actions();
            for (keep, caption) in [
                (true, MsgId::WriterKeepDrafts),
                (false, MsgId::WriterUseFile),
            ] {
                choices = choices.child(
                    Button::new()
                        .radio(*choice.read() == Some(keep))
                        .on_press(move |_| choice.set(Some(keep)))
                        .child(text(caption)),
                );
            }
            let primary = crate::design::SubmitAction {
                id: action_id,
                caption: text(MsgId::WriterCompareReturn),
                enabled: choice.read().is_some(),
                action: EventHandler::new(move |()| {
                    let Some(keep) = *choice.peek() else {
                        return;
                    };
                    let result = state
                        .write()
                        .catalogue
                        .as_mut()
                        .map(|catalogue| catalogue.accept_external(keep, &expected));
                    if let Some(result) = result {
                        if result.is_ok() {
                            let mut value = state.write();
                            value.comparison = None;
                            value.view = value.comparison_return;
                        }
                        message.report(result, text(MsgId::WriterCompared));
                    }
                }),
            };
            let shortcut = primary.clone();
            body = body
                .on_global_key_down(move |event: Event<KeyboardEventData>| {
                    if !state.peek().modal_open() && crate::design::keyboard::submit_key(&event) {
                        event.prevent_default();
                        event.stop_propagation();
                        shortcut.run();
                    }
                })
                .child(label().text(text(MsgId::WriterComparisonResolution)))
                .child(choices)
                .child(primary.button());
        } else {
            body = body.child(
                Button::new()
                    .on_press(move |_| prepare(writer))
                    .child(text(MsgId::WriterCompare)),
            );
        }
        ScrollView::new()
            .width(Size::fill())
            .height(Size::flex(1.))
            .child(body)
    }
}
pub(super) fn prepare(mut writer: Writer) {
    let result = writer
        .localisation
        .peek()
        .catalogue
        .as_ref()
        .map(|catalogue| catalogue.compare());
    match result {
        Some(Ok(comparison)) => {
            let mut state = writer.localisation.write();
            if state.view != CatalogueView::Compare {
                state.comparison_return = state.view;
            }
            state.comparison = Some(comparison);
            state.panel = None;
            state.view = CatalogueView::Compare;
        }
        Some(Err(error)) => writer.message.error(error),
        None => {}
    }
}
