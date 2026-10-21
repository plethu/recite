use super::*;
use crate::tests::support::{self, worker::Worker};
use std::fs;

#[test]
fn actual_save_worker_publishes_accepted_source_and_keeps_typing_arriving_during_the_write() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Accepted before save.".into());
        model.apply()
    });
    let accepted = writer.session.model().document().source_snapshot();
    let mut worker = Worker::new(writer, worker());
    worker.state.save();
    worker.state.change(|model| {
        model.set_draft("Newer field typing.".into());
        Ok(())
    });
    while worker.state.work.busy() {
        worker.next();
    }
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        accepted.as_ref()
    );
    assert_eq!(worker.state.session.model().draft(), "Newer field typing.");
    assert!(worker.state.session.model().has_draft());
    worker.close();
}
