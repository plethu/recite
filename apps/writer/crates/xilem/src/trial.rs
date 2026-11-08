//! Frontend views and native glue over `recite_writer_app::trial`.
pub(crate) use recite_writer_app::trial::*;
#[cfg(test)]
mod tests;

use crate::Writer;
use recite_compiler::authoring::CancellationToken;
use recite_writer_session::preview::Request;
use std::sync::{Arc, mpsc};
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |state: &mut Writer, sender| state.trial.sender = Some(sender),
        Writer::accept_trial,
    )
}

async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Arc<Request>>) {
    // The UI admits one request until its output is acknowledged, including after Cancel.
    // This bounds the toolkit's unbounded channel without creating a second task system.
    while let Some(request) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                request.prepare(&CancellationToken::new())
            }))
            .unwrap_or_else(|_| {
                Err("Preview preparation stopped unexpectedly. Restart to retry.".into())
            });
            let (receipt, returned) = mpsc::sync_channel(1);
            if proxy
                .message(Output {
                    request,
                    result,
                    receipt,
                })
                .is_ok()
            {
                let retired = returned.recv().ok().and_then(|(request, retired)| {
                    drop(request);
                    retired
                });
                match retired {
                    Some(Retired::Prepared(prepared)) => drop(prepared),
                    Some(Retired::Preview(preview)) => drop(preview),
                    None => {}
                }
            }
        })
        .await;
    }
}
