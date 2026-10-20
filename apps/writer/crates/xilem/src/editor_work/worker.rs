use super::*;
use recite_writer_session::project::save::sequence::Receipt;
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

struct Output(Result<Receipt, ()>);
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorOutput").finish_non_exhaustive()
    }
}
pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| writer.work.sender = Some(sender),
        |writer: &mut Writer, Output(result)| match result {
            Ok(receipt) => writer.accept_save_work(receipt),
            Err(()) => {
                let error = FileError::Commit(std::io::Error::other(
                    "The editor worker stopped unexpectedly.",
                ));
                if let Some(Operation::Saving(mut sequence)) = writer.work.operation.take() {
                    sequence.failed(error);
                    writer.drive_save(sequence);
                } else {
                    writer.finish_save_work(Err(error));
                }
            }
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Work>) {
    // The sequence admits one request until the UI has accepted its receipt.
    // Closing a window is guarded while any durable operation is outstanding.
    while let Some(work) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work.execute()))
                .map_err(|_| ());
            let _ = proxy.message(Output(result));
        })
        .await;
    }
}
