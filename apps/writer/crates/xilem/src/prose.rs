//! Native glue over `recite_writer_app::prose`: one blocking check at a time, newest first.
use super::Writer;
use recite_writer_app::prose::{Checked, Checker, Request};
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

/// A finished check; `None` when the checker stopped unexpectedly.
struct Output(Option<Checked>);
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("ProseOutput").field(&self.0).finish()
    }
}

pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| {
            writer.prose.sender = Some(sender);
            writer.check_prose_if_needed();
        },
        |writer: &mut Writer, Output(checked)| {
            if let Some(checked) = checked {
                writer.accept_prose(checked);
            }
        },
    )
}

async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Request>) {
    // The checker, and the project checkers it builds on first use, live on this worker.
    let mut checker = Checker::default();
    while let Some(mut request) = requests.recv().await {
        // Only the newest source matters; requests queued behind it are already stale.
        while let Ok(newer) = requests.try_recv() {
            request = newer;
        }
        let mut owned = std::mem::take(&mut checker);
        let finished = tokio::task::spawn_blocking(move || {
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                request.execute(&mut owned)
            }))
            .ok();
            (checked.is_some().then_some(owned), checked)
        })
        .await;
        let checked = match finished {
            Ok((owned, checked)) => {
                checker = owned.unwrap_or_default();
                checked
            }
            Err(_) => None,
        };
        if proxy.message(Output(checked)).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests;
