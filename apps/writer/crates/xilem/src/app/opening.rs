//! The native project worker runs shell jobs: cold loads, folder choice, creation and activation.
use super::App;
use recite_writer_app::shell::opening::loading::load;
pub(crate) use recite_writer_app::shell::opening::*;
#[cfg(target_os = "linux")]
use recite_writer_session::activation::ActivationInbox;
use std::sync::{Arc, mpsc};
use tokio::{sync::mpsc::UnboundedReceiver, task::JoinHandle};
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

pub(super) fn worker() -> impl View<App, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |app: &mut App, sender| app.connect_project_worker(sender),
        |app: &mut App, output| app.accept_project_output(output),
    )
}
async fn serve(proxy: MessageProxy<Output>, mut jobs: UnboundedReceiver<Job>) {
    let mut interval = tokio::time::interval(std::time::Duration::from_millis(30));
    let mut previous_open: Option<JoinHandle<()>> = None;
    #[cfg(target_os = "linux")]
    let mut inbox: Option<ActivationInbox> = None;
    loop {
        tokio::select! {
            job = jobs.recv() => {
                let Some(job) = job else { break; };
                match job {
                    #[cfg(target_os = "linux")]
                    Job::Listen(next) => inbox = Some(next),
                    Job::Open(request) => {
                        previous_open = Some(tokio::spawn(open_after(previous_open.take(), proxy.clone(), request)));
                    }
                    Job::Browse => { let selected = rfd::AsyncFileDialog::new().set_title("Choose project folder").pick_folder().await; let _ = proxy.message(Output::Chosen(selected.map(|file| file.path().to_owned()))); }
                    Job::Create { path, name } => {
                        let proxy = proxy.clone();
                        tokio::task::spawn_blocking(move || {
                            let result = recite_config::create_starter_project(&path, &name).map(|_| path).map_err(|error| error.to_string());
                            let _ = proxy.message(Output::Created(result));
                        });
                    }
                }
            }
            _ = interval.tick() => {
                #[cfg(target_os = "linux")]
                if let Some(request) = inbox.as_ref().and_then(ActivationInbox::try_next)
                    && proxy.message(Output::Activation(Box::new(request))).is_err() { break; }
            }
        }
    }
}

async fn open_after(
    previous: Option<JoinHandle<()>>,
    proxy: MessageProxy<Output>,
    request: Arc<Request>,
) {
    // Reopening waits for the previous owner's recovery lease to retire.
    // Await outside serve's loop so activation keeps receiving replies.
    if let Some(previous) = previous {
        let _ = previous.await;
    }
    let _ = tokio::task::spawn_blocking(move || {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| load(&request)))
            .unwrap_or_else(|_| {
                Err("Project loading stopped unexpectedly. Reopen the project to retry.".into())
            });
        let (retire, retired) = mpsc::sync_channel(1);
        if proxy
            .message(Output::Opened(Opened {
                request,
                result,
                retire,
            }))
            .is_ok()
        {
            drop(retired.recv());
        }
    })
    .await;
}

#[cfg(test)]
mod tests;
