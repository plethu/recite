use super::*;
use crate::tests::support::{self, worker::Worker};
use std::{fs, path::Path};

#[test]
fn native_worker_retires_rejected_navigation_before_retry_and_cleans_up_accepted_reload() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(
        &other,
        ":: other default\n> other@aaaaaaaaaaaaaaaaaaaa\n  Other.\n-> END\n",
    )
    .unwrap();
    let mut worker = Worker::new(support::open(&directory).unwrap(), worker());
    worker.next(); // The worker supplies the runtime needed by recovery checkpoints.
    worker.state.switch(&other);
    worker.state.cancel_navigation();
    while worker.state.navigation.busy() {
        worker.next();
    }
    assert_eq!(
        worker.state.session.project().unwrap().current().path(),
        directory.path().join("scene.recite")
    );
    worker.state.switch(&other);
    while worker.state.navigation.busy() {
        worker.next();
    }
    assert_eq!(
        worker.state.session.project().unwrap().current().path(),
        other
    );
    fs::write(
        &other,
        ":: other default\n> other@aaaaaaaaaaaaaaaaaaaa\n  External.\n-> END\n",
    )
    .unwrap();
    worker.state.reload();
    while worker.state.navigation.busy() {
        worker.next();
    }
    assert_eq!(worker.state.session.model().draft(), "External.");
    assert!(!worker.state.session.model().document().can_undo());
    drop(worker.close());
    let reopened = recite_writer_session::ProjectSession::open(
        directory.path(),
        Some(Path::new("other.recite")),
    )
    .unwrap();
    assert_eq!(reopened.current().model().draft(), "External.");
}
