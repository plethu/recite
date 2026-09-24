//! Source submission shares the dialog and routed-editor shortcut policy.
use crate::commands::CommandExt;
use crate::{
    design::{Button, SubmitAction},
    editing::Writer,
    messages::{MsgId, text},
};
use freya::prelude::*;
use recite_writer_model::Workbench;
#[derive(Clone)]
pub(super) struct SourceActions {
    pub writer: Writer,
    pub completion: Element,
}
impl PartialEq for SourceActions {
    fn eq(&self, other: &Self) -> bool {
        self.writer.dark == other.writer.dark
    }
}
impl Component for SourceActions {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let compact = *writer.layout.available.read() < 1100. * crate::design::tokens::ui_scale();
        let changed = writer
            .buffers
            .model
            .read()
            .as_ref()
            .is_ok_and(|m| m.has_draft());
        let primary = SubmitAction {
            id: use_a11y(),
            caption: text(MsgId::WriterApplyDraft),
            enabled: changed,
            action: EventHandler::new(move |()| writer.perform(Workbench::apply)),
        };
        let shortcut = primary.clone();
        crate::design::actions()
            .main_align(Alignment::Start)
            .padding(crate::design::tokens::SPACE_SM)
            .background(crate::design::tokens::colors().inset)
            .on_global_key_down(move |event: Event<KeyboardEventData>| {
                if !writer.localisation.peek().modal_open()
                    && !*writer.settings_open.peek()
                    && writer.command_search.mode.peek().is_none()
                    && crate::design::keyboard::submit_key(&event)
                {
                    event.prevent_default();
                    event.stop_propagation();
                    shortcut.run();
                }
            })
            .maybe_child((!compact).then(|| crate::commands::Command::Rename.button(writer)))
            .maybe_child((!compact).then(|| crate::commands::Command::Declarations.button(writer)))
            .child(self.completion.clone())
            .child(primary.button())
            .child(
                Button::new()
                    .named(text(MsgId::WriterDiscardDraft))
                    .enabled(changed)
                    .on_press(move |_| {
                        writer.perform(|m| {
                            m.discard();
                            Ok(())
                        })
                    })
                    .child(text(MsgId::WriterDiscardDraft)),
            )
    }
}
