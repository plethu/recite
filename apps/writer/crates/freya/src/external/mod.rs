mod handoff;
mod watch;
use crate::{
    design::{Button, ComparisonRow, ComparisonView, SubmitAction, tokens as t},
    editing::{Pane, Writer},
    messages::{MsgId, text},
};
use freya::prelude::*;
pub(crate) use handoff::Handoff;
pub(crate) use watch::Watch;
pub(crate) fn open(mut writer: Writer) {
    if let Err(error) = try_open(writer) {
        writer.message.error(error);
    }
}
pub(crate) fn try_open(mut writer: Writer) -> Result<(), String> {
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
        let mut writer = self.writer;
        let mut choice = use_state(|| None::<bool>);
        let id = use_a11y();
        let files = writer.files.read();
        let Some(comparison) = files.as_ref().and_then(|p| p.external.as_ref()) else {
            return rect();
        };
        let submit = SubmitAction {
            id,
            caption: text(MsgId::WriterUseEditableDraft),
            enabled: choice.read().is_some(),
            action: EventHandler::new(move |()| {
                let Some(keep) = *choice.peek() else {
                    return;
                };
                writer.buffers.harvest();
                let result = {
                    let mut model = writer.buffers.model.write();
                    let mut files = writer.files.write();
                    match (model.as_mut(), files.as_mut()) {
                        (Ok(model), Some(files)) => files
                            .resolve_external(model, keep)
                            .map_err(|e| e.to_string()),
                        _ => Err("Open a project first.".into()),
                    }
                };
                match result {
                    Ok(copy) => {
                        writer.buffers.sync(writer.dark);
                        writer.pane.set(Pane::Map);
                        writer.message.info(format!(
                            "Editable draft restored. Previous session retained at {}",
                            copy.display()
                        ));
                    }
                    Err(e) => writer.message.error(e),
                }
            }),
        };
        let keyboard = submit.clone();
        let body = rect()
            .width(Size::fill())
            .padding(t::SPACE_XL)
            .spacing(t::SPACE_LG)
            .on_key_down(move |e: Event<KeyboardEventData>| {
                if crate::design::keyboard::submit_key(&e) {
                    e.prevent_default();
                    e.stop_propagation();
                    keyboard.run();
                }
            })
            .child(
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
                    ),
            )
            .child(label().text(comparison.path.display().to_string()))
            .child(ComparisonView {
                code: true,
                before: text(MsgId::WriterDraftVersion),
                after: text(MsgId::WriterDiskVersion),
                rows: ComparisonRow::source("", &comparison.draft, &comparison.disk),
            })
            .child(
                rect()
                    .horizontal()
                    .spacing(t::SPACE_MD)
                    .child(
                        Button::new()
                            .radio(*choice.read() == Some(true))
                            .on_press(move |_| choice.set(Some(true)))
                            .child(text(MsgId::WriterDraftVersion)),
                    )
                    .child(
                        Button::new()
                            .radio(*choice.read() == Some(false))
                            .on_press(move |_| choice.set(Some(false)))
                            .child(text(MsgId::WriterDiskVersion)),
                    ),
            )
            .child(
                paragraph()
                    .width(Size::fill())
                    .span(Span::new(text(MsgId::WriterDiskDraftHint))),
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
#[derive(Clone, Copy)]
pub(crate) struct WatchPoll {
    pub writer: Writer,
}
impl PartialEq for WatchPoll {
    fn eq(&self, _: &Self) -> bool {
        false
    }
}
impl Component for WatchPoll {
    fn render(&self) -> impl IntoElement {
        let mut tick = freya::sdk::use_timeout(|| std::time::Duration::from_millis(500));
        if tick.elapsed() {
            tick.reset();
            poll_external_changes(self.writer);
        }
        rect()
    }
}

fn poll_external_changes(mut writer: Writer) {
    let result = writer
        .files
        .peek()
        .as_ref()
        .and_then(|project| project.handoff.as_ref())
        .and_then(Handoff::poll);
    if let Some(result) = result {
        writer
            .message
            .report(result, text(MsgId::WriterExternalOpened));
        if let Some(project) = writer.files.write().as_mut() {
            project.handoff = None;
        }
    }
    let files = writer.files.peek();
    let Some(project) = files.as_ref() else {
        return;
    };
    let Ok(watch) = &project.watch else {
        return;
    };
    let paths = match watch.changed() {
        Ok(paths) => paths,
        Err(error) => {
            writer.message.error(format!(
                "File watching stopped: {error}. Save still checks for external changes."
            ));
            return;
        }
    };
    let Some(target) = paths.iter().find(|path| project.externally_changed(path)) else {
        return;
    };
    if writer.message.is_error() {
        return;
    }
    let target = target.clone();
    writer.message.error_with_action(
        text(MsgId::WriterDiskChanged),
        text(MsgId::WriterCompare),
        EventHandler::new(move |()| compare_changed_document(writer, &target)),
    );
}

fn compare_changed_document(mut writer: Writer, target: &std::path::Path) {
    match writer
        .buffers
        .switch(writer.files, target, writer.dark, |_| Ok(()))
    {
        Ok(()) => open(writer),
        Err(error) => writer.message.error(error),
    }
}
pub(crate) use handoff::open_editor;
