//! Frontend views and native glue over `recite_writer_app::navigation::worker`.
use super::*;
pub(crate) use recite_writer_app::navigation::worker::*;
#[cfg(test)]
mod tests;
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

enum Output {
    Ready(tokio::runtime::Handle),
    Prepared {
        completed: Box<Completed>,
        retire: mpsc::SyncSender<Option<ResultValue>>,
    },
    Retired(Arc<Request>),
    ReloadCleanupFailed(String),
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationOutput").finish_non_exhaustive()
    }
}

pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| writer.navigation.sender = Some(sender),
        |writer: &mut Writer, output: Output| match output {
            Output::Ready(runtime) => writer.navigation.runtime = Some(runtime),
            Output::Prepared { completed, retire } => {
                let retired = writer.accept_navigation(*completed);
                let _ = retire.send(retired);
            }
            Output::Retired(request) => writer.navigation_retired(&request),
            Output::ReloadCleanupFailed(message) => writer.reload_cleanup_failed(message),
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Job>) {
    if proxy
        .message(Output::Ready(tokio::runtime::Handle::current()))
        .is_err()
    {
        return;
    }
    while let Some(job) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let completed = Box::new(job.execute());
            let request = completed.request.clone();
            let (retire, retired) = mpsc::sync_channel(1);
            if proxy
                .message(Output::Prepared { completed, retire })
                .is_ok()
            {
                if let Ok(Some(value)) = retired.recv()
                    && let Err(message) = value.retire()
                {
                    let _ = proxy.message(Output::ReloadCleanupFailed(message));
                }
                let _ = proxy.message(Output::Retired(request));
            }
        })
        .await;
    }
}
