use super::*;
use recite_writer_session::external::Handoff;
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

struct Output(Arc<Request>, Option<Result<(), String>>);
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorLauncherOutput")
            .finish_non_exhaustive()
    }
}
pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| writer.external.sender = Some(sender),
        |writer: &mut Writer, Output(request, result)| writer.accept_editor(request, result),
    )
}
async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Arc<Request>>) {
    while let Some(request) = requests.recv().await {
        let result = match Handoff::configured(request.path().to_owned(), request.editor.as_ref()) {
            Ok(handoff) => {
                if proxy.message(Output(Arc::clone(&request), None)).is_err() {
                    return;
                }
                loop {
                    if let Some(result) = handoff.poll() {
                        break result;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                }
            }
            Err(error) => Err(error),
        };
        // Dropping this receiver during teardown never abandons the independently reaped process.
        if proxy.message(Output(request, Some(result))).is_err() {
            return;
        }
    }
}
