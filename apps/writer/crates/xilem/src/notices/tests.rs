use super::*;
use crate::tests::support;
use crate::workspaces::tests::ToolDriver;
use recite_writer_model::View;
use std::fs;
#[path = "catalogue/tests.rs"]
mod catalogue;
#[path = "visual/tests.rs"]
mod visual;

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";

fn connect(writer: &mut Writer) -> mpsc::Receiver<Request> {
    let (send, requests) = mpsc::channel(1);
    let (catalogue, _) = watch::channel(None);
    writer.notices.connection = Some(Connection {
        requests: send,
        catalogue,
    });
    writer.notices.flight = None;
    requests
}
fn changed(writer: &mut Writer, paths: impl IntoIterator<Item = PathBuf>) {
    let mut changes = Changes::default();
    for path in paths {
        changes.affected.insert(path);
    }
    writer.accept_external_notice(Output {
        window: writer.window,
        event: Event::Sources(changes),
    });
    writer.resume_external_notices();
}
fn finish(writer: &mut Writer, requests: &mut mpsc::Receiver<Request>) {
    let observed = requests.try_recv().unwrap().observe();
    writer.accept_external_notice(Output {
        window: writer.window,
        event: Event::Observed(observed),
    });
    writer.resume_external_notices();
}
#[test]
fn atomic_replacement_and_deletion_are_advisory_and_keep_unapplied_drafts() {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut requests = connect(&mut writer);
    writer
        .session
        .edit(|model| {
            model.select(View::Passage("11111111111111111111".into()))?;
            model.set_draft("Keep this exact pending draft.".into());
            Ok(())
        })
        .unwrap();
    let path = directory.path().join("scene.recite");
    let replacement = directory.path().join("replacement");
    fs::write(&replacement, SOURCE.replace("Hello.", "Disk version.")).unwrap();
    fs::rename(replacement, &path).unwrap();
    changed(&mut writer, [path.clone()]);
    finish(&mut writer, &mut requests);
    assert_eq!(writer.notices.found.len(), 1);
    assert_eq!(
        writer.session.model().draft(),
        "Keep this exact pending draft."
    );
    assert_eq!(writer.session.model().document().source(), SOURCE);
    fs::remove_file(&path).unwrap();
    changed(&mut writer, [path]);
    finish(&mut writer, &mut requests);
    assert_eq!(writer.notices.found.len(), 1);
    assert_eq!(
        writer.session.model().draft(),
        "Keep this exact pending draft."
    );
}
#[test]
fn save_races_and_retired_flights_recheck_the_current_saved_baseline() {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut requests = connect(&mut writer);
    let path = directory.path().join("scene.recite");
    changed(&mut writer, [path.clone()]);
    let old = requests.try_recv().unwrap().observe();
    writer
        .session
        .edit(|model| {
            model.set_draft("Accepted own save.".into());
            model.apply()
        })
        .unwrap();
    support::save(&mut writer).unwrap();
    writer.accept_external_notice(Output {
        window: writer.window,
        event: Event::Observed(old),
    });
    writer.resume_external_notices();
    assert!(writer.notices.found.is_empty());
    assert!(writer.notices.flight.is_some());
    finish(&mut writer, &mut requests);
    assert!(writer.notices.found.is_empty());
    changed(&mut writer, [path]);
    let mut retired = requests.try_recv().unwrap().observe();
    retired.flight = Flight(Arc::new(()));
    writer.accept_external_notice(Output {
        window: writer.window,
        event: Event::Observed(retired),
    });
    assert!(
        writer.notices.flight.is_some(),
        "a foreign flight cannot consume current work"
    );
    assert!(writer.notices.found.is_empty());
}

#[test]
fn requests_wait_through_saving_without_consuming_all_open_rescans() {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut requests = connect(&mut writer);
    writer.session.parts_mut().unwrap().0.saving = true;
    writer.notices.pending.sources = Affected::AllOpen;
    writer.resume_external_notices();
    assert!(requests.try_recv().is_err());
    assert!(matches!(writer.notices.pending.sources, Affected::AllOpen));
    writer.session.parts_mut().unwrap().0.saving = false;
    writer.resume_external_notices();
    assert_eq!(requests.try_recv().unwrap().sources.len(), 1);
}

#[test]
fn checkout_and_changes_during_a_flight_resume_without_losing_paths() {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut requests = connect(&mut writer);
    let path = directory.path().join("scene.recite");
    let mut checkout = writer.session.checkout().unwrap();
    fs::write(&path, SOURCE.replace("Hello.", "Changed during checkout.")).unwrap();
    changed(&mut writer, [path.clone()]);
    assert!(requests.try_recv().is_err());
    writer
        .session
        .restore_checkout(&mut checkout, None)
        .unwrap();
    writer.resume_external_notices();
    let first = requests.try_recv().unwrap().observe();
    fs::write(&path, SOURCE).unwrap();
    changed(&mut writer, [path]);
    assert!(requests.try_recv().is_err(), "only one flight is admitted");
    writer.accept_external_notice(Output {
        window: writer.window,
        event: Event::Observed(first),
    });
    writer.resume_external_notices();
    assert_eq!(writer.notices.found.len(), 1);
    finish(&mut writer, &mut requests);
    assert!(
        writer.notices.found.is_empty(),
        "the queued second observation sees restored disk bytes"
    );
}

#[test]
fn a_queued_compare_cannot_follow_retired_evidence_or_requeue_closed_files_forever() {
    let directory = support::project(SOURCE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(&other, SOURCE.replace("Hello.", "Other.")).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer
        .session
        .project_mut()
        .unwrap()
        .switch(&other)
        .unwrap();
    let path = directory.path().join("scene.recite");
    fs::write(&path, SOURCE.replace("Hello.", "External.")).unwrap();
    let mut requests = connect(&mut writer);
    changed(&mut writer, [path.clone()]);
    finish(&mut writer, &mut requests);
    assert_eq!(writer.notices.found.len(), 1);
    writer
        .session
        .project_mut()
        .unwrap()
        .close_document(&path)
        .unwrap();
    writer.compare_noticed_change();
    assert!(!writer.navigation.busy());
    assert_eq!(writer.session.project().unwrap().current().path(), other);
    writer.resume_external_notices();
    assert!(writer.notices.found.is_empty());
    assert!(writer.notices.pending.sources.is_empty());
    writer.resume_external_notices();
    assert!(requests.try_recv().is_err());
}

#[test]
fn comparing_a_retained_source_uses_normal_navigation_and_preserves_both_documents_drafts() {
    let directory = support::project(SOURCE).unwrap();
    let path = directory.path().join("scene.recite");
    let other = directory.path().join("other.recite");
    fs::write(
        &other,
        SOURCE.replace("11111111111111111111", "22222222222222222222"),
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Retained draft.".into());
        Ok(())
    });
    writer
        .session
        .project_mut()
        .unwrap()
        .switch(&other)
        .unwrap();
    writer.change(|model| {
        model.set_draft("Outgoing draft.".into());
        Ok(())
    });
    fs::write(&path, SOURCE.replace("Hello.", "External.")).unwrap();
    let mut observations = connect(&mut writer);
    changed(&mut writer, [path.clone()]);
    finish(&mut writer, &mut observations);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    let mut tools = ToolDriver::new(&mut writer);
    writer.compare_noticed_change();
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    tools.finish(&mut writer);
    assert!(
        writer.workspaces.screen == crate::workspaces::Screen::External,
        "{}",
        writer.status
    );
    let project = writer.session.project().unwrap();
    assert_eq!(project.current().path(), path);
    assert_eq!(project.current().model().draft(), "Retained draft.");
    assert_eq!(
        project
            .sessions()
            .find(|session| session.path() == other)
            .unwrap()
            .model()
            .draft(),
        "Outgoing draft."
    );
    assert_eq!(project.current().model().document().source(), SOURCE);
    assert!(
        writer.status.starts_with("Disk comparison ready."),
        "{}",
        writer.status
    );
}
