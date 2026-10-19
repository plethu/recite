//! Directory events demand a file hash, never a PO parse on the UI thread.
#[cfg(test)]
mod tests;
use recite_core::po::PoDocumentFingerprint;
use std::{path::PathBuf, thread::JoinHandle};
pub(super) struct Request {
    pub path: PathBuf,
    pub baseline: PoDocumentFingerprint,
}
pub(super) struct Output {
    pub request: Request,
    pub fingerprint: Result<PoDocumentFingerprint, String>,
}
pub(super) struct Worker {
    requests: async_channel::Sender<Request>,
    waiting: async_channel::Receiver<Request>,
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
    pub fn submit(&mut self, request: Request) -> std::io::Result<()> {
        if self.thread.as_ref().is_none_or(JoinHandle::is_finished) {
            let waiting = self.waiting.clone();
            let output = self.output.clone();
            self.thread = Some(
                std::thread::Builder::new()
                    .name("writer-catalogue-watch".into())
                    .spawn(move || {
                        while let Ok(request) = waiting.recv_blocking() {
                            let fingerprint = crate::project::read_regular(&request.path)
                                .map(|source| PoDocumentFingerprint::from_source(&source))
                                .map_err(|e| e.to_string());
                            if output
                                .send_blocking(Output {
                                    request,
                                    fingerprint,
                                })
                                .is_err()
                            {
                                break;
                            }
                        }
                    })?,
            );
        }
        self.requests
            .force_send(request)
            .map(drop)
            .map_err(|_| std::io::Error::other("Catalogue watcher stopped"))
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.requests.close();
        self.results.close();
    }
}
