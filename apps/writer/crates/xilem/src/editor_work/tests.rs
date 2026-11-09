#![allow(unused_imports)]
use super::super::{Phase, Writer, feedback};
use super::*;
use crate::external_editor::linked::Link;
use crate::tests::support::{finish_work, open, project};
use recite_writer_model::EditOperation;
use recite_writer_session::project::save::sequence::Work;
use tokio::sync::mpsc as async_mpsc;
use tokio::sync::mpsc::UnboundedSender;

#[test]
fn save_stays_responsive_and_retains_typing_that_arrived_after_disk_dispatch() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.change(|model| {
        model.set_draft("Accepted before saving.".into());
        Ok(())
    });
    writer.save();
    assert!(writer.work.busy());
    let write = loop {
        let work = requests.try_recv().unwrap();
        if matches!(work, Work::Write(_)) {
            break work;
        }
        writer.accept_save_work(work.execute());
    };
    let receipt = write.execute();
    writer.change(|model| {
        model.set_draft("Newer typing during save.".into());
        Ok(())
    });
    writer.request_close();
    assert!(writer.keep_running());
    assert!(writer.work.busy());
    writer.accept_save_work(receipt);
    finish_work(&mut writer, &mut requests).unwrap();
    assert!(!writer.work.busy());
    assert_eq!(writer.session.model().draft(), "Newer typing during save.");
    assert!(writer.session.dirty());
    assert!(writer.status.contains("newer edits remain"));
    let source =
        std::fs::read_to_string(writer.session.project().unwrap().current().path()).unwrap();
    assert!(source.contains("Accepted before saving."));
    assert!(!source.contains("Newer typing during save."));
}

#[test]
fn typing_during_apply_saves_captured_source_and_preserves_the_live_draft() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.change(|model| {
        model.set_draft("Captured field draft.".into());
        Ok(())
    });
    writer.save_and_close();
    let work = requests.try_recv().unwrap();
    assert!(matches!(work, Work::Edit(_)));
    let receipt = work.execute();
    writer.change(|model| {
        model.set_draft("Typed before Apply returned.".into());
        Ok(())
    });
    writer.accept_save_work(receipt);
    finish_work(&mut writer, &mut requests).unwrap();
    assert!(writer.keep_running());
    assert!(writer.phase() == Phase::ConfirmClose);
    assert!(!writer.work.busy());
    assert_eq!(
        writer.session.model().draft(),
        "Typed before Apply returned."
    );
    let saved =
        std::fs::read_to_string(writer.session.project().unwrap().current().path()).unwrap();
    assert!(saved.contains("Captured field draft."));
    assert!(!saved.contains("Typed before Apply returned."));
    assert!(writer.status.contains("newer edits remain"));
}

#[test]
fn unavailable_worker_keeps_unsaved_work_and_releases_busy_guard() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Keep me.".into());
        Ok(())
    });
    writer.save();
    assert!(!writer.work.busy());
    assert!(!writer.session.project().unwrap().files().saving);
    assert!(writer.status.contains("worker is unavailable"));
    assert_eq!(writer.session.model().draft(), "Keep me.");
}
