use super::*;
use freya::code_editor::Rope;
use recite_core::SourcePosition;
use recite_writer_model::Document;
use std::{sync::Barrier, time::Duration};

fn request(suffix: usize) -> Arc<Request> {
    let source = format!(":: start default\n-> dest\n:: destination\n-> END\n# {suffix}\n");
    let document = Document::new(&source).unwrap();
    Arc::new(Request {
        analysis: document.completion_snapshot(),
        draft: Rope::from_str(&source),
        position: SourcePosition::new(2, 8).unwrap(),
    })
}
#[test]
fn replacement_and_cancel_cover_queued_and_already_claimed_jobs() {
    let mut worker = Worker::new();
    let barrier = Arc::new(Barrier::new(2));
    let held = Arc::clone(&barrier);
    worker.worker = Some(thread::spawn(move || {
        held.wait();
    }));
    worker.submit(request(1)).unwrap();
    let queued = worker.state.lock().unwrap().current.clone().unwrap();
    worker.submit(request(2)).unwrap();
    assert!(queued.checkpoint().is_err());
    assert_eq!(worker.waiting.len(), 1);
    assert_eq!(worker.waiting.capacity(), Some(1));
    let claimed = worker.waiting.try_recv().unwrap();
    // Cancellation also covers a job claimed before any worker-side registration.
    worker.submit(request(3)).unwrap();
    assert!(claimed.control.checkpoint().is_err());
    let latest = worker.state.lock().unwrap().current.clone().unwrap();
    worker.cancel();
    assert!(latest.checkpoint().is_err());
    assert!(worker.waiting.is_empty());
    assert!(worker.results.is_empty());
    barrier.wait();
    worker.worker.take().unwrap().join().unwrap();
}

#[test]
fn an_older_completion_cannot_clear_the_newer_jobs_cancellation_authority() {
    let mut worker = Worker::new();
    let barrier = Arc::new(Barrier::new(2));
    let held = Arc::clone(&barrier);
    worker.worker = Some(thread::spawn(move || {
        held.wait();
    }));
    worker.submit(request(1)).unwrap();
    let old = worker.waiting.try_recv().unwrap();
    worker.submit(request(2)).unwrap();
    let latest = worker.state.lock().unwrap().current.clone().unwrap();
    publish(
        &worker.state,
        &worker.output,
        &old.control,
        Output::Failed {
            request: Some(old.request),
            message: "obsolete failure".into(),
        },
    );
    assert!(worker.results.is_empty());
    worker.cancel();
    assert!(latest.checkpoint().is_err());
    barrier.wait();
    worker.worker.take().unwrap().join().unwrap();
}
#[test]
fn rapid_requests_publish_only_the_latest_owned_result_and_release_on_drop() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    let first = request(0);
    let obsolete = Arc::downgrade(&first);
    worker.submit(first).unwrap();
    let mut latest = request(1);
    for index in 1..40 {
        latest = request(index);
        worker.submit(Arc::clone(&latest)).unwrap();
    }
    let mut result = None;
    for _ in 0..5_000 {
        if let Ok(Output::Completed(completed)) = receiver.try_recv()
            && Arc::ptr_eq(&completed.request, &latest)
        {
            result = Some(completed);
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let result = result.unwrap_or_else(|| panic!("latest completion did not settle"));
    assert!(
        result
            .items
            .candidates()
            .iter()
            .any(|candidate| candidate.name() == "destination")
    );
    assert!(result.items.source().ends_with("# 39\n"));
    assert!(obsolete.upgrade().is_none());
    let retained = Arc::downgrade(&latest);
    drop((result, latest));
    let state = Arc::clone(&worker.state);
    let handle = worker.worker.take().unwrap();
    drop(worker);
    handle.join().unwrap();
    assert!(state.lock().unwrap().status == Status::Closed);
    assert!(retained.upgrade().is_none());
}

#[test]
fn failure_wakes_the_retained_receiver_and_allows_immediate_retry() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    let barrier = Arc::new(Barrier::new(2));
    let held = Arc::clone(&barrier);
    let (exited, old_exit) = std::sync::mpsc::channel();
    worker.worker = Some(thread::spawn(move || {
        held.wait();
        exited.send(()).unwrap();
    }));
    assert!(!worker.worker.as_ref().unwrap().is_finished());
    failed(&worker.state, &worker.waiting, &worker.output);
    assert!(matches!(
        receiver.try_recv(),
        Ok(Output::Failed { request: None, .. })
    ));
    assert!(worker.state.lock().unwrap().status == Status::Failed);
    assert!(!worker.worker.as_ref().unwrap().is_finished());
    // A second unread failure must also be cleared by immediate retry.
    failed(&worker.state, &worker.waiting, &worker.output);
    assert_eq!(receiver.len(), 1);
    worker.submit(request(10)).unwrap();
    barrier.wait();
    old_exit.recv_timeout(Duration::from_secs(1)).unwrap();
    for _ in 0..5_000 {
        if let Ok(Output::Completed(completed)) = receiver.try_recv() {
            assert!(completed.items.source().ends_with("# 10\n"));
            return;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("retry did not settle");
}

#[test]
fn cancelling_a_queued_result_releases_it_and_drop_closes_the_retained_receiver() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    let input = request(42);
    let retained = Arc::downgrade(&input);
    worker.submit(input).unwrap();
    for _ in 0..5_000 {
        if !receiver.is_empty() {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert_eq!(receiver.len(), 1);
    assert_eq!(receiver.capacity(), Some(1));
    worker.cancel();
    assert!(receiver.is_empty());
    let handle = worker.worker.take().unwrap();
    drop(worker);
    handle.join().unwrap();
    assert!(retained.upgrade().is_none());
    assert!(matches!(
        receiver.try_recv(),
        Err(async_channel::TryRecvError::Closed)
    ));
}
