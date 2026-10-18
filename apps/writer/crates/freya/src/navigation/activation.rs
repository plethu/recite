//! Apply forwarded links on the UI thread through the same route owner as history.
use super::*;

pub(crate) fn receive(
    writer: Writer,
    request: crate::activation::IncomingRoute,
    ready: bool,
) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    if request.cancelled.load(Ordering::Acquire) {
        return Err("Writer activation was cancelled".into());
    }
    if writer.localisation.peek().modal_open()
        || *writer.settings_open.peek()
        || crate::design::modal_open()
        || writer.command_search.mode.peek().is_some()
    {
        return Err("Close the current dialog before opening a link.".into());
    }
    if let Some(route) = &request.route {
        let location = route.parse::<Location>().map_err(str::to_owned)?;
        if !ready || writer.files.peek().is_none() {
            return Err("The writer is still opening a project.".into());
        }
        writer.buffers.harvest();
        if writer
            .buffers
            .model
            .peek()
            .as_ref()
            .is_ok_and(|model| model.view() == &View::Source && model.has_draft())
        {
            return Err("Save or discard the Source draft before opening a link.".into());
        }
        if writer.localisation.peek().dirty() {
            return Err(crate::localisation::close_drafts_message());
        }
        let root = writer
            .files
            .peek()
            .as_ref()
            .map(|f| f.root().display().to_string())
            .unwrap_or_default();
        let mut message = writer.message;
        super::route(
            writer,
            location,
            Some(request.cancelled),
            EventHandler::new(move |result: Result<(), String>| {
                let result = result.map_err(|error| format!("Project {root}: {error}"));
                if let Err(error) = &result {
                    message.error(error.clone());
                }
                if result.is_ok() {
                    let router = RouterContext::get();
                    let next = snapshot(writer);
                    let current = router.current::<Location>();
                    if next != current {
                        if next.same_place(&current) {
                            let _ = router.replace(next);
                        } else {
                            let _ = router.push(next);
                        }
                    }
                    focus();
                }
                let _ = request.reply.send(result);
            }),
        )?;
    } else {
        focus();
        let _ = request.reply.send(Ok(()));
    }
    Ok(())
}
fn focus() {
    #[cfg(not(test))]
    Platform::get().focus_window(Platform::window_id());
}
