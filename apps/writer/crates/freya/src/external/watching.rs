//! File watching reports conflicts; only explicit comparison admits a resolution.
use super::*;
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
    if writer.files.peek().as_ref().is_some_and(|p| p.saving) {
        return;
    }
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
    let changes = watch.changed();
    if let Some(error) = changes.error {
        writer.message.error(format!(
            "File watching stopped: {error}. Save still checks for external changes."
        ));
    }
    let Some(observation) = project
        .external_checks(&changes.affected)
        .into_iter()
        .map(|request| request.observe())
        .find(|observation| observation.changed())
    else {
        return;
    };
    if writer.message.is_error() {
        return;
    }
    let target = observation.path().to_owned();
    writer.message.error_with_action(
        text(MsgId::WriterDiskChanged),
        text(MsgId::WriterCompare),
        EventHandler::new(move |()| compare_changed_document(writer, &target)),
    );
}

fn compare_changed_document(mut writer: Writer, target: &std::path::Path) {
    if let Err(error) = writer.open_document_then(
        target,
        EventHandler::new(move |result: Result<(), String>| match result {
            Ok(()) => open(writer),
            Err(error) => writer.message.error(error),
        }),
    ) {
        writer.message.error(error);
    }
}
