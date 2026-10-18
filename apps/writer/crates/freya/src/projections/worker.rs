//! One lazy preparing thread, one queued source and one unacknowledged receipt.
use recite_writer_model::{PreparedProjection, ProjectionRequest};
use std::{
    io,
    thread::{self, JoinHandle},
};
pub(super) struct Output {
    pub request: ProjectionRequest,
    pub result: Result<PreparedProjection, String>,
    pub receipt: async_channel::Sender<(ProjectionRequest, Option<PreparedProjection>)>,
}
pub(super) struct Worker {
    requests: async_channel::Sender<ProjectionRequest>,
    waiting: async_channel::Receiver<ProjectionRequest>,
    output: async_channel::Sender<Output>,
    results: async_channel::Receiver<Output>,
    thread: Option<JoinHandle<()>>,
}
impl Worker {
    pub fn new() -> Self {
        let (requests, waiting) = async_channel::bounded(1);
        let (output, results) = async_channel::bounded(1);
        Self {
            requests,
            waiting,
            output,
            results,
            thread: None,
        }
    }
    pub fn results(&self) -> async_channel::Receiver<Output> {
        self.results.clone()
    }
    pub fn submit(&mut self, request: ProjectionRequest) -> io::Result<()> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            let waiting = self.waiting.clone();
            let output = self.output.clone();
            let results = self.results.clone();
            self.thread = Some(
                thread::Builder::new()
                    .name("writer-projection".into())
                    .spawn(move || run(&waiting, &output, &results))?,
            );
        }
        self.requests
            .force_send(request)
            .map(drop)
            .map_err(|_| io::Error::other("Script worker closed"))
    }
}
fn run(
    waiting: &async_channel::Receiver<ProjectionRequest>,
    output: &async_channel::Sender<Output>,
    results: &async_channel::Receiver<Output>,
) {
    while let Ok(request) = waiting.recv_blocking() {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| request.prepare()))
            .map_err(|_| {
                "Script preparation stopped unexpectedly. Reopen the scene to retry.".to_owned()
            })
            .and_then(|result| result.map_err(|e| e.to_string()));
        let (receipt, returned) = async_channel::bounded(1);
        if output
            .send_blocking(Output {
                request,
                result,
                receipt,
            })
            .is_err()
        {
            break;
        }
        // Drop result arrays on this thread even when the root unmounts.
        let retired = futures_lite::future::block_on(futures_lite::future::race(
            async { returned.recv().await.ok() },
            async {
                output.closed().await;
                None
            },
        ));
        drop(retired);
        if output.is_closed() {
            while let Ok(output) = results.try_recv() {
                drop(output);
            }
            break;
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.requests.close();
        self.results.close();
        // Parsing is finite but not interruptible; detach rather than stall UI teardown.
    }
}

#[cfg(test)]
mod tests;
