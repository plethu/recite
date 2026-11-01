use super::*;
use std::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};
#[cfg(test)]
mod tests;

struct Output {
    completed: Completed,
    retire: mpsc::SyncSender<Retirement>,
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectToolsOutput").finish_non_exhaustive()
    }
}
pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| writer.workspaces.tools.sender = Some(sender),
        |writer: &mut Writer, Output { completed, retire }| {
            let _ = retire.send(writer.accept_tool(completed));
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Job>) {
    while let Some(job) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let mut completed = job.execute();
            loop {
                let request = completed.request.clone();
                let (retire, retired) = mpsc::sync_channel(1);
                if proxy.message(Output { completed, retire }).is_err() {
                    break;
                }
                let next = retired
                    .recv()
                    .ok()
                    .and_then(|retirement| retirement.finish(request));
                match next {
                    Some(next) => completed = next,
                    None => break,
                }
            }
        })
        .await;
    }
}
