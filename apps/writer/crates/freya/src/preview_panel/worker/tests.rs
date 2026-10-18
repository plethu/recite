use super::super::{Snapshot, prepare::Inputs};
use super::*;
use recite_writer_model::Workbench;
use std::{sync::Barrier, time::Duration};

fn request(number: usize) -> Arc<Request> {
    let model = Workbench::new(&format!(
        ":: start default\n> guide@12345678901234567890\n  Hello {number}.\n-> END\n"
    ))
    .unwrap();
    Arc::new(Request {
        source: model.preview_request().unwrap(),
        view: model.view().clone(),
        inputs: Inputs {
            locale: String::new(),
            variant: String::new(),
            include_drafts: false,
            values: Default::default(),
            catalogue: None,
        },
        snapshot: Snapshot::capture(String::new(), false, None),
    })
}
fn receive(receiver: &async_channel::Receiver<Output>) -> Output {
    for _ in 0..5_000 {
        if let Ok(output) = receiver.try_recv() {
            return output;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("preview did not settle");
}

#[test]
fn replacement_and_cancel_cover_claimed_and_queued_requests() {
    let mut worker = Worker::new();
    let barrier = Arc::new(Barrier::new(2));
    let held = barrier.clone();
    worker.thread = Some(thread::spawn(move || {
        held.wait();
    }));
    worker.submit(request(1)).unwrap();
    let claimed = worker.waiting.try_recv().unwrap();
    worker.submit(request(2)).unwrap();
    assert!(claimed.control.checkpoint().is_err());
    worker.submit(request(3)).unwrap();
    assert_eq!(worker.waiting.len(), 1);
    assert_eq!(worker.waiting.capacity(), Some(1));
    let queued = worker.waiting.try_recv().unwrap();
    worker.cancel();
    assert!(queued.control.checkpoint().is_err());
    barrier.wait();
    worker.thread.take().unwrap().join().unwrap();
}
#[test]
fn result_receipts_bound_retention_and_release_before_latest_job() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    worker.submit(request(0)).unwrap();
    let first = receive(&receiver);
    assert!(first.result.is_ok());
    let mut latest = request(1);
    for i in 1..40 {
        latest = request(i);
        worker.submit(latest.clone()).unwrap();
    }
    assert_eq!(worker.waiting.len(), 1);
    assert!(receiver.is_empty(), "worker must await ownership receipt");
    let prepared = first.result.unwrap_or_else(|e| panic!("{e}"));
    first
        .receipt
        .try_send((first.request, Some(Retired::Prepared(prepared))))
        .unwrap_or_else(|_| panic!("receipt"));
    let last = receive(&receiver);
    assert!(Arc::ptr_eq(&last.request, &latest));
    assert!(last.result.is_ok());
    last.receipt
        .try_send((last.request, None))
        .unwrap_or_else(|_| panic!("receipt"));
}
#[test]
fn closing_with_an_unconsumed_output_releases_the_thread_and_large_result() {
    let mut worker = Worker::new();
    let first = request(0);
    let retained = Arc::downgrade(&first);
    worker.submit(first).unwrap();
    for _ in 0..5_000 {
        if !worker.results.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(!worker.results.is_empty(), "preview did not publish");
    let handle = worker.thread.take().unwrap();
    drop(worker);
    for _ in 0..5_000 {
        if handle.is_finished() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(handle.is_finished(), "shutdown stranded the receipt waiter");
    handle.join().unwrap();
    assert!(retained.upgrade().is_none());
}

#[test]
fn preparation_failure_returns_ownership_and_allows_a_clean_retry() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    let mut invalid = request(1);
    Arc::get_mut(&mut invalid).unwrap().inputs.variant = "invalid&variant".into();
    worker.submit(invalid).unwrap();
    let failed = receive(&receiver);
    assert!(failed.result.is_err());
    failed
        .receipt
        .try_send((failed.request, None))
        .unwrap_or_else(|_| panic!("receipt"));
    worker.submit(request(2)).unwrap();
    let retried = receive(&receiver);
    assert!(retried.result.is_ok());
    retried
        .receipt
        .try_send((retried.request, None))
        .unwrap_or_else(|_| panic!("receipt"));
}
