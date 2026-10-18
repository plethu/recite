use super::*;
use recite_writer_model::Workbench;

#[test]
fn old_and_cancelled_receipts_cannot_clear_a_newer_pending_trial() {
    let model = Workbench::new(recite_writer_model::FIXTURE).unwrap();
    let first = Arc::new(super::super::tests::capture(&model, None, false));
    let latest = Arc::new(super::super::tests::capture(&model, None, false));
    let mut work = Work {
        worker: Worker::new(),
        pending: Some(latest.clone()),
    };
    assert!(!work.finish(&first));
    assert!(Arc::ptr_eq(work.pending.as_ref().unwrap(), &latest));
    assert!(work.finish(&latest));
    assert!(work.pending.is_none());
    assert!(!work.finish(&latest));
    work.pending = Some(first.clone());
    work.worker.cancel();
    work.pending = None;
    assert!(!work.finish(&first));
}
