use super::*;
use recite_writer_model::Document;
use std::time::Duration;
fn request(n: usize) -> ProjectionRequest {
    Document::new(format!(
        ":: start default\n> line@11111111111111111111\n  Hello {n}.\n-> END\n"
    ))
    .unwrap()
    .projection_request()
}
fn receive(receiver: &async_channel::Receiver<Output>) -> Output {
    for _ in 0..5_000 {
        if let Ok(output) = receiver.try_recv() {
            return output;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("projection did not settle");
}
#[test]
fn queued_sources_coalesce_while_one_receipt_owns_the_result() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    worker.submit(request(0)).unwrap();
    let first = receive(&receiver);
    assert!(first.result.is_ok());
    let mut latest = request(1);
    for n in 1..40 {
        latest = request(n);
        worker.submit(latest.clone()).unwrap();
    }
    assert_eq!(worker.waiting.len(), 1);
    assert!(receiver.is_empty());
    first
        .receipt
        .try_send((first.request, first.result.ok()))
        .unwrap_or_else(|_| panic!("receipt"));
    let last = receive(&receiver);
    assert!(last.request.same_source(&latest));
    last.receipt
        .try_send((last.request, last.result.ok()))
        .unwrap_or_else(|_| panic!("receipt"));
    worker.requests.close();
    worker.thread.take().unwrap().join().unwrap();
}
#[test]
fn invalid_source_reports_failure_and_worker_recovers() {
    let mut worker = Worker::new();
    let receiver = worker.results();
    worker
        .submit(
            Document::new(":: broken\n? invalid\n")
                .unwrap()
                .projection_request(),
        )
        .unwrap();
    let invalid = receive(&receiver);
    assert!(invalid.result.is_err());
    invalid
        .receipt
        .try_send((invalid.request, None))
        .unwrap_or_else(|_| panic!("receipt"));
    worker.submit(request(1)).unwrap();
    let valid = receive(&receiver);
    assert!(valid.result.is_ok());
    // Closing with an outstanding result must not strand the worker.
    worker.results.close();
    worker.thread.take().unwrap().join().unwrap();
}
