mod handoff;
mod resolution;
mod watching;
pub(crate) use watching::WatchPoll;
#[cfg(test)]
mod tests;

use crate::design::keyboard;
use crate::{
    design::{Button, ComparisonRow, ComparisonView, SubmitAction, tokens as t},
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) use recite_writer_session::external::Handoff;
pub(crate) use recite_writer_session::external::Watch;
use recite_writer_session::project::ExternalDecision;
pub(crate) fn open(mut writer: Writer) {
    if let Err(error) = try_open(writer) {
        writer.message.error(error);
    }
}
pub(crate) fn try_open(mut writer: Writer) -> Result<(), String> {
    writer.work.require_idle()?;
    writer.require_project_edit_ready()?;
    writer.buffers.harvest();
    let result = {
        let mut model = writer.buffers.model.write();
        let mut files = writer.files.write();
        match (model.as_mut(), files.as_mut()) {
            (Ok(model), Some(files)) => files.compare_external(model).map_err(|e| e.to_string()),
            _ => Err("Open a project first.".into()),
        }
    };
    result?;
    writer.buffers.sync(writer.dark);
    writer.pane.set(Pane::Disk);
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) struct ExternalScreen {
    pub writer: Writer,
}
impl PartialEq for ExternalScreen {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for ExternalScreen {
    fn render(&self) -> impl IntoElement {
        let writer = self.writer;
        let choice = use_state(|| None::<ExternalDecision>);
        let id = use_a11y();
        let files = writer.files.read();
        let Some(comparison) = files.as_ref().and_then(|p| p.external.as_ref()) else {
            return rect();
        };
        let submit = SubmitAction {
            id,
            caption: "Confirm inspected version".into(),
            enabled: choice.read().is_some(),
            action: EventHandler::new(move |()| {
                let Some(decision) = *choice.peek() else {
                    return;
                };
                resolution::resolve(writer, decision);
            }),
        };
        let keyboard = submit.clone();
        let body = rect()
            .width(Size::fill())
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if keyboard::submit_key(&e) {
                    e.prevent_default();
                    e.stop_propagation();
                    keyboard.run();
                }
            })
            .child(header(writer))
            .child(label().text(comparison.path().display().to_string()))
            .maybe_child(comparison.archive().map(|path| paragraph().width(Size::fill()).span(Span::new(format!("Complete recovery copy: {}", path.display())))))
            .maybe_child(comparison.pending_draft().map(|_| paragraph().width(Size::fill()).span(Span::new("An unapplied field draft is retained. Keep Writer version preserves it; Use inspected disk version archives it before replacing the document."))))
            .child(ComparisonView {
                code: true,
                before: "Accepted Writer source".into(),
                after: text(MsgId::WriterDiskVersion),
                rows: ComparisonRow::source("", comparison.writer_source(), comparison.disk_source()),
            })
            .child(decisions(choice))
            .child(
                paragraph()
                    .width(Size::fill())
                    .span(Span::new("This decision does not write the source file. Keep Writer version allows the next Save to replace the inspected disk version. A complete recovery copy is retained before either decision.")),
            )
            .child(submit.button());
        rect().width(Size::fill()).height(Size::fill()).child(
            ScrollView::new()
                .width(Size::fill())
                .height(Size::fill())
                .child(body),
        )
    }
}
pub(crate) use handoff::open_editor;

fn header(mut writer: Writer) -> impl IntoElement {
    rect()
        .horizontal()
        .spacing(t::SPACE_LG)
        .child(
            label()
                .text(text(MsgId::WriterCompare))
                .font_size(t::title()),
        )
        .child(
            Button::new()
                .flat()
                .on_press(move |_| writer.pane.set(Pane::Map))
                .child(text(MsgId::WriterReturnWriting)),
        )
}

fn decisions(mut choice: State<Option<ExternalDecision>>) -> impl IntoElement {
    rect()
        .horizontal()
        .spacing(t::SPACE_MD)
        .child(
            Button::new()
                .named("Keep Writer version")
                .radio(*choice.read() == Some(ExternalDecision::KeepWriter))
                .on_press(move |_| choice.set(Some(ExternalDecision::KeepWriter)))
                .child("Keep Writer version"),
        )
        .child(
            Button::new()
                .named("Use inspected disk version")
                .radio(*choice.read() == Some(ExternalDecision::UseInspectedDisk))
                .on_press(move |_| choice.set(Some(ExternalDecision::UseInspectedDisk)))
                .child("Use inspected disk version"),
        )
}
