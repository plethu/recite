//! Debouncing owns its receive before awaiting: cancelling the service select cannot lose it.
use recite_writer_session::external::{Changes, Subscription};
use std::time::Duration;
use tokio::{sync::mpsc, task::JoinHandle};

pub struct Listener {
    pub changes: mpsc::Receiver<Changes>,
    task: JoinHandle<()>,
}
impl Listener {
    pub fn new(source: Subscription) -> Self {
        let (send, changes) = mpsc::channel(1);
        let task = tokio::spawn(async move {
            while let Some(mut batch) = source.next().await {
                tokio::time::sleep(Duration::from_millis(500)).await;
                batch.merge(source.changed());
                if send.send(batch).await.is_err() {
                    break;
                }
            }
        });
        Self { changes, task }
    }
}
impl Drop for Listener {
    fn drop(&mut self) {
        self.task.abort();
    }
}
pub async fn next(listener: &mut Option<Listener>) -> Option<Changes> {
    match listener {
        Some(listener) => listener.changes.recv().await,
        None => std::future::pending().await,
    }
}
