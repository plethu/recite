//! One lazy worker. Capacity-one mailboxes coalesce queued trials; receipts return
//! rejected/replaced sessions to the worker for destruction, never to the UI.
use super::request::Request;
use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{PreparedPreview, Preview};
use std::{
    io,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

pub(super) enum Retired {
    Prepared(Box<PreparedPreview>),
    Preview(Box<Preview>),
}
pub(super) struct Output {
    pub request: Arc<Request>,
    pub result: Result<Box<PreparedPreview>, String>,
    pub receipt: async_channel::Sender<(Arc<Request>, Option<Retired>)>,
}
struct Job {
    request: Arc<Request>,
    control: CancellationToken,
}
pub(super) struct Worker {
    current: Arc<Mutex<Option<CancellationToken>>>,
    thread: Option<JoinHandle<()>>,
    requests: async_channel::Sender<Job>,
    waiting: async_channel::Receiver<Job>,
    output: async_channel::Sender<Output>,
    results: async_channel::Receiver<Output>,
}
impl Worker {
    pub fn new() -> Self {
        let (requests, waiting) = async_channel::bounded(1);
        let (output, results) = async_channel::bounded(1);
        Self {
            current: Arc::default(),
            thread: None,
            requests,
            waiting,
            output,
            results,
        }
    }
    pub fn results(&self) -> async_channel::Receiver<Output> {
        self.results.clone()
    }
    pub fn submit(&mut self, request: Arc<Request>) -> io::Result<()> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            let waiting = self.waiting.clone();
            let output = self.output.clone();
            let current = self.current.clone();
            let results = self.results.clone();
            self.thread = Some(
                thread::Builder::new()
                    .name("writer-preview".into())
                    .spawn(move || run(&current, &waiting, &output, &results))?,
            );
        }
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let control = CancellationToken::new();
        if let Some(old) = current.replace(control.clone()) {
            old.interrupt();
        }
        let displaced = self.requests.force_send(Job { request, control });
        drop(current);
        displaced
            .map(drop)
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "Preview worker closed"))
    }
    pub fn cancel(&self) {
        let mut current = self
            .current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(old) = current.take() {
            old.interrupt();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        self.requests.close();
        self.results.close();
        // Detach: an active compiler phase is not cooperatively interruptible yet.
    }
}
fn run(
    current: &Mutex<Option<CancellationToken>>,
    waiting: &async_channel::Receiver<Job>,
    output: &async_channel::Sender<Output>,
    results: &async_channel::Receiver<Output>,
) {
    while let Ok(job) = waiting.recv_blocking() {
        if job.control.checkpoint().is_err() {
            continue;
        }
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let setup = job.request.inputs.prepare(&job.request.source)?;
            if job.control.checkpoint().is_err() {
                return Err(String::new());
            }
            job.request.source.prepare(setup).map_err(|e| e.to_string())
        }))
        .unwrap_or_else(|_| {
            Err("Preview preparation stopped unexpectedly. Restart to retry.".into())
        });
        let (receipt, returned) = async_channel::bounded(1);
        let guard = current
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if job.control.checkpoint().is_err() {
            drop(guard);
            continue;
        }
        let sent = output.force_send(Output {
            request: job.request,
            result,
            receipt,
        });
        drop(guard);
        drop(sent);
        // The UI returns ownership even for stale results. Closing the component
        // drops its receipt, so teardown cannot strand this worker.
        let retired = futures_lite::future::block_on(futures_lite::future::race(
            async { returned.recv().await.ok() },
            async {
                output.closed().await;
                None
            },
        ));
        match retired.and_then(|(request, retired)| {
            drop(request);
            retired
        }) {
            Some(Retired::Prepared(prepared)) => drop(prepared),
            Some(Retired::Preview(preview)) => drop(preview),
            None => {}
        }
        if output.is_closed() {
            while let Ok(pending) = results.try_recv() {
                drop(pending);
            }
            break;
        }
    }
}

#[cfg(test)]
mod tests;
