use super::*;
use recite_compiler::authoring::Interrupted;
use recite_writer_session::localisation::catalogue::Catalogue;

#[test]
fn cancelled_or_retired_destination_receipts_restore_both_original_owners_and_retire_candidates() {
    for retired in [false, true] {
        let (directory, mut writer) = fixture();
        writer
            .follow_location(&location(Screen::Translations))
            .unwrap();
        let original = writer.localisation.watch_scope().unwrap();
        let candidate = directory.path().join("other.po");
        fs::write(&candidate, PO.replace("Bonjour.", "Candidate.")).unwrap();
        let (_runtime, mut jobs) = connect(&mut writer);
        let mut target = location(Screen::Entry);
        target.catalogue = Some("other.po".into());
        let waiting = activate(&mut writer, target);
        publish(&mut writer, jobs.try_recv().unwrap());
        assert!(writer.session.checked_out());
        let completed = jobs.try_recv().unwrap().execute();
        if retired {
            writer.navigation.pending = None;
        } else {
            writer.cancel_navigation();
        }
        if let Some(receipt) = writer.accept_navigation(completed) {
            receipt.retire().unwrap();
        }
        assert!(!writer.session.checked_out());
        assert!(writer.localisation.watch_scope().as_ref() == Some(&original));
        assert!(writer.localisation.watch_request().is_some());
        assert_eq!(
            writer.localisation.catalogue_path(),
            Some(directory.path().join("fr.po").as_path())
        );
        assert!(
            Catalogue::open_recoverable(&candidate).is_ok(),
            "the rejected candidate lease must retire"
        );
        if !retired {
            assert!(waiting.try_recv().unwrap().is_err());
        }
    }
}

#[test]
fn failed_destination_enqueue_returns_exclusive_owners_and_keeps_source_typing() {
    let (directory, mut writer) = fixture();
    writer
        .follow_location(&location(Screen::Translations))
        .unwrap();
    writer
        .session
        .edit(|model| {
            model.set_draft("Keep pending prose.".into());
            Ok(())
        })
        .unwrap();
    let original = writer.localisation.watch_scope().unwrap();
    let (_runtime, _jobs) = connect(&mut writer);
    let target = location(Screen::Entry);
    let plan = Plan::capture(&writer, &target, false, None).unwrap();
    let request = Arc::new(Request {
        target: Target::Present(target),
        origin: Origin::capture(&writer),
        cancel: CancellationToken::new(),
        forwarded: None,
    });
    let work = plan.checkout(&mut writer).unwrap();
    let (sender, receiver) = async_mpsc::unbounded_channel();
    drop(receiver);
    writer.navigation.sender = Some(sender);
    assert!(
        writer
            .send_navigation(request, worker::Work::Present(Box::new(work)))
            .is_err()
    );
    assert!(!writer.session.checked_out());
    assert_eq!(writer.session.model().draft(), "Keep pending prose.");
    assert!(writer.localisation.watch_scope().as_ref() == Some(&original));
    assert!(writer.localisation.watch_request().is_some());
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
}

struct Panic;
impl WorkControl for Panic {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        panic!("injected destination failure")
    }
}
#[test]
fn preparation_panic_keeps_both_owners_and_source_draft_with_explicit_project_quarantine() {
    let (_directory, mut writer) = fixture();
    writer
        .follow_location(&location(Screen::Translations))
        .unwrap();
    writer
        .session
        .edit(|model| {
            model.set_draft("Recover this source draft.".into());
            Ok(())
        })
        .unwrap();
    let original = writer.localisation.watch_scope().unwrap();
    let target = location(Screen::Entry);
    let work = Plan::capture(&writer, &target, false, None)
        .unwrap()
        .checkout(&mut writer)
        .unwrap();
    let mut prepared = work.execute(&Panic);
    prepared.restore(&mut writer).unwrap();
    assert!(prepared.present(&mut writer).is_err());
    assert!(!writer.session.checked_out());
    assert!(writer.session.recovery_reason() == Some(RecoveryReason::ReopenRequired));
    assert!(writer.localisation.watch_scope().as_ref() == Some(&original));
    assert_eq!(writer.session.model().draft(), "Recover this source draft.");
}

#[test]
fn failed_entry_preparation_returns_an_activation_error_after_restoring_owners() {
    let (_directory, mut writer) = fixture();
    writer.follow_location(&location(Screen::Entry)).unwrap();
    let original = writer.localisation.watch_scope().unwrap();
    let (_runtime, mut jobs) = connect(&mut writer);
    let mut target = location(Screen::Entry);
    target.entry = Some("missing-context".into());
    let waiting = activate(&mut writer, target);
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    assert!(
        waiting
            .try_recv()
            .unwrap()
            .unwrap_err()
            .contains("entry is unavailable")
    );
    assert!(!writer.session.read_only());
    assert!(writer.localisation.watch_scope().as_ref() == Some(&original));
    assert_eq!(writer.current_location().screen, Screen::Write);
}

#[test]
fn cancelled_disk_link_keeps_the_previously_reviewed_comparison() {
    let (directory, mut writer) = fixture();
    writer.follow_location(&location(Screen::Disk)).unwrap();
    let original = writer
        .session
        .project()
        .unwrap()
        .files()
        .external
        .as_ref()
        .unwrap()
        .disk_source()
        .to_owned();
    fs::write(
        directory.path().join("scene.recite"),
        SOURCE.replace("Hello.", "New external change."),
    )
    .unwrap();
    let (_runtime, mut jobs) = connect(&mut writer);
    let waiting = activate(&mut writer, location(Screen::Disk));
    publish(&mut writer, jobs.try_recv().unwrap());
    let prepared = jobs.try_recv().unwrap().execute();
    writer.cancel_navigation();
    if let Some(receipt) = writer.accept_navigation(prepared) {
        receipt.retire().unwrap();
    }
    assert!(waiting.try_recv().unwrap().is_err());
    assert_eq!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .external
            .as_ref()
            .unwrap()
            .disk_source(),
        original
    );
}
