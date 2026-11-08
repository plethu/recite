use super::*;
use crate::Phase;
use crate::tests::support::worker::Worker;
use recite_writer_session::declarations::DeclarationAction;

#[test]
fn actual_project_worker_completes_declaration_recovery_before_releasing_close_guard() {
    let (directory, writer, _) = super::super::tests::fixture();
    let mut worker = Worker::new(writer, worker());
    worker.state.declaration_action(DeclarationAction::Bind(
        directory.path().join("schema.toml"),
    ));
    worker.next();
    assert!(worker.state.project_jobs_busy());
    assert!(worker.state.declaration_session().unwrap().busy());
    worker.state.request_close();
    assert!(worker.state.phase == Phase::Writing);
    worker.next();
    assert!(!worker.state.project_jobs_busy());
    assert!(!worker.state.declaration_session().unwrap().busy());
    assert!(worker.state.declaration_session().unwrap().current());
    worker.close();
}
