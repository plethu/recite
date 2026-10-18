//! Install a finished load only while its captured source and intent remain current.
use super::{Loading, can_switch_project};
use freya::prelude::WritableUtils;

pub(super) fn poll(loading: &Loading) {
    let loaded = loading.job.peek().as_ref().and_then(|job| {
        job.load
            .try_result()
            .map(|result| (job.load.cancelled(), job.source.clone(), result))
    });
    let Some((cancelled, source, loaded)) = loaded else {
        return;
    };
    let current = loading
        .writer
        .buffers
        .model
        .peek()
        .as_ref()
        .ok()
        .map(|model| model.document().source_snapshot());
    let mut job = loading.job;
    let mut files = loading.files;
    let mut panel = loading.panel;
    let mut message = loading.writer.message;
    #[cfg(target_os = "linux")]
    let activation = job.write().take().and_then(|mut job| job.activation.take());
    #[cfg(not(target_os = "linux"))]
    job.set(None);
    #[cfg(target_os = "linux")]
    let activation_cancelled = activation
        .as_ref()
        .is_some_and(|request| request.cancelled.load(std::sync::atomic::Ordering::Acquire));
    #[cfg(not(target_os = "linux"))]
    let activation_cancelled = false;
    if cancelled || activation_cancelled {
        message.info("Project opening cancelled.".into());
        #[cfg(target_os = "linux")]
        reply_error(activation, "Project opening cancelled".into());
        return;
    }
    if source != current || !can_switch_project(loading.writer) {
        message.error(
            "The current document changed while loading. Save it before opening another project."
                .into(),
        );
        #[cfg(target_os = "linux")]
        reply_error(
            activation,
            "The current document changed while loading".into(),
        );
        return;
    }
    let super::worker::LoadedProject {
        files: project,
        workbench,
        view,
    } = match loaded {
        Ok(loaded) => loaded,
        Err(error) => {
            panel.set(true);
            message.error(error.to_string());
            #[cfg(target_os = "linux")]
            reply_error(activation, error.to_string());
            return;
        }
    };
    let recovered = project.has_recovery();
    let opened_root = project.root().display().to_string();
    let reset = {
        let mut state = loading.writer.localisation;
        let mut localisation = state.write();
        localisation.install(None).map(|()| {
            *localisation = crate::localisation::Localisation::default();
        })
    };
    if let Err(error) = reset {
        message.error(error.clone());
        #[cfg(target_os = "linux")]
        reply_error(activation, error);
        return;
    }
    loading.writer.reset_scenes();
    loading
        .writer
        .buffers
        .install(workbench, loading.writer.dark);
    files.set(Some(project));
    // An explicit route is fixed intent; preferences may change during the load.
    let view = view.unwrap_or(loading.writer.preferences.peek().config.writer.view);
    if let Err(error) = loading.writer.scene_opened(view) {
        let error =
            format!("Project {opened_root} opened, but its initial scene could not open: {error}");
        message.error(error.clone());
        #[cfg(target_os = "linux")]
        reply_error(activation, error);
        return;
    }
    panel.set(false);
    message.info(
        if recovered {
            "Recovered your previous session."
        } else {
            "Project opened."
        }
        .into(),
    );
    #[cfg(target_os = "linux")]
    if let Some(request) = activation {
        let reply = request.reply.clone();
        if let Err(error) = crate::navigation::activation::receive(loading.writer, request, true) {
            let error = format!(
                "Project {opened_root} opened, but the target location could not open: {error}"
            );
            message.error(error.clone());
            let _ = reply.send(Err(error));
        }
    }
}

#[cfg(target_os = "linux")]
fn reply_error(activation: Option<crate::activation::IncomingRoute>, error: String) {
    if let Some(request) = activation {
        let _ = request.reply.send(Err(error));
    }
}
