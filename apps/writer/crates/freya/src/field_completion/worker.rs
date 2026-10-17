//! One lazy worker with capacity-one request/result channels and latest-request cancellation.
use super::request::{Completed, Request};
use recite_compiler::authoring::{AuthoringError, CancellationToken};
use recite_writer_model::EditError;
use std::{
    io,
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

struct Job {
    request: Arc<Request>,
    control: CancellationToken,
}
pub(super) enum Output {
    Completed(Arc<Completed>),
    Failed {
        request: Option<Arc<Request>>,
        message: String,
    },
}
#[derive(Default, PartialEq)]
enum Status {
    #[default]
    Running,
    Failed,
    Closed,
}
#[derive(Default)]
struct State {
    // Assigned before enqueueing: cancellation covers queued AND already-claimed work.
    // Finishing a job never clears this token: it may belong to a newer request.
    current: Option<CancellationToken>,
    status: Status,
}
pub(super) struct Worker {
    state: Arc<Mutex<State>>,
    worker: Option<JoinHandle<()>>,
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
            state: Arc::default(),
            worker: None,
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
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if state.status == Status::Failed
            || self.worker.as_ref().is_none_or(JoinHandle::is_finished)
        {
            let state = Arc::clone(&self.state);
            let waiting = self.waiting.clone();
            let output = self.output.clone();
            self.worker = Some(
                thread::Builder::new()
                    .name("writer-completion".into())
                    .spawn(move || run_guarded(&state, &waiting, &output))?,
            );
        }
        let control = CancellationToken::new();
        if let Some(previous) = state.current.replace(control.clone()) {
            previous.interrupt();
        }
        state.status = Status::Running;
        let old = self.requests.force_send(Job { request, control });
        let result = self.results.try_recv().ok();
        drop(state);
        drop(result);
        old.map(drop).map_err(|_| {
            io::Error::new(
                io::ErrorKind::BrokenPipe,
                "Source suggestions worker closed",
            )
        })
    }
    pub fn cancel(&self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(current) = state.current.take() {
            current.interrupt();
        }
        let old = (self.waiting.try_recv().ok(), self.results.try_recv().ok());
        drop(state);
        drop(old);
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.status = Status::Closed;
        if let Some(current) = state.current.take() {
            current.interrupt();
        }
        self.requests.close();
        self.results.close();
        let old = (self.waiting.try_recv().ok(), self.results.try_recv().ok());
        drop(state);
        drop(old);
        // Dropping the handle detaches; cancellation releases active analysis off the GUI thread.
    }
}
fn run_guarded(
    state: &Mutex<State>,
    waiting: &async_channel::Receiver<Job>,
    output: &async_channel::Sender<Output>,
) {
    if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(state, waiting, output)))
        .is_err()
    {
        failed(state, waiting, output);
    }
}
fn failed(
    state: &Mutex<State>,
    waiting: &async_channel::Receiver<Job>,
    output: &async_channel::Sender<Output>,
) {
    let mut state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if state.status == Status::Closed {
        return;
    }
    state.status = Status::Failed;
    state.current = None;
    let old = output.force_send(Output::Failed {
        request: None,
        message: "Source suggestions stopped unexpectedly. Close and reopen suggestions to retry."
            .into(),
    });
    let pending = waiting.try_recv().ok();
    drop(state);
    drop((old, pending));
}
fn run(
    state: &Mutex<State>,
    waiting: &async_channel::Receiver<Job>,
    output: &async_channel::Sender<Output>,
) {
    while let Ok(job) = waiting.recv_blocking() {
        if job.control.checkpoint().is_err() {
            continue;
        }
        let draft = String::from(&job.request.draft);
        let completed =
            match job
                .request
                .analysis
                .complete(&draft, job.request.position, &job.control)
            {
                Ok(items) => Output::Completed(Arc::new(Completed {
                    request: Arc::clone(&job.request),
                    items,
                })),
                Err(EditError::Authoring(AuthoringError::Interrupted(_))) => continue,
                Err(error) => Output::Failed {
                    request: Some(Arc::clone(&job.request)),
                    message: error.to_string(),
                },
            };
        publish(state, output, &job.control, completed);
    }
}
fn publish(
    state: &Mutex<State>,
    output: &async_channel::Sender<Output>,
    control: &CancellationToken,
    completed: Output,
) {
    // Serialize publication with submit/cancel so cancelled work cannot refill the channel.
    let state = state
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let old = if state.status != Status::Closed && control.checkpoint().is_ok() {
        Some(output.force_send(completed))
    } else {
        None
    };
    drop(state);
    drop(old);
}

#[cfg(test)]
mod tests;
