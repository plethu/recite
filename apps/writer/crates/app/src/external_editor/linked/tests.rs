use super::*;
use crate::test_support as support;
use recite_writer_session::location::Location;
use recite_writer_session::project::save::sequence::Work;
use tokio::sync::mpsc as async_mpsc;

fn source_location() -> Location {
    Location {
        document: "scene.recite".into(),
        source: true,
        ..Location::default()
    }
}
fn reply() -> (IncomingRoute, mpsc::Receiver<Result<(), String>>) {
    let (sender, waiting) = mpsc::channel();
    (
        IncomingRoute {
            project: None,
            route: None,
            document: None,
            editor: None,
            cancelled: Arc::new(atomic::AtomicBool::new(false)),
            reply: sender,
        },
        waiting,
    )
}
fn connect(
    writer: &mut Writer,
) -> (
    tokio::runtime::Runtime,
    async_mpsc::UnboundedReceiver<Work>,
    async_mpsc::UnboundedReceiver<Arc<Request>>,
) {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    writer.navigation.runtime = Some(runtime.handle().clone());
    let (sender, saves) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    let (sender, launches) = async_mpsc::unbounded_channel();
    writer.external.sender = Some(sender);
    (runtime, saves, launches)
}

#[test]
fn cold_and_hot_source_links_save_then_launch_without_entering_the_source_editor() {
    for cold in [true, false] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        let (incoming, waiting) = reply();
        let (runtime, mut saves, mut launches);
        if cold {
            writer.follow_location(&source_location()).unwrap();
            assert!(writer.external.waiting.is_some());
            assert!(
                writer.attach_linked_editor_reply(Some(incoming.reply), Some(incoming.cancelled))
            );
            (runtime, saves, launches) = connect(&mut writer);
        } else {
            (runtime, saves, launches) = connect(&mut writer);
            let (sender, mut navigation) = async_mpsc::unbounded_channel();
            writer.navigation.sender = Some(sender);
            writer
                .navigate_activation(source_location(), incoming)
                .unwrap();
            support::finish_navigation(&mut writer, &mut navigation).unwrap();
        }
        assert!(
            waiting.try_recv().is_err(),
            "the route must wait for the OS launcher"
        );
        assert!(writer.session.model().view() != &recite_writer_model::View::Source);
        writer.resume_linked_editor();
        support::finish_work(&mut writer, &mut saves).unwrap();
        let request = launches.try_recv().unwrap();
        assert_eq!(request.path(), directory.path().join("scene.recite"));
        assert!(waiting.try_recv().is_err());
        request.finish_link(Ok(()));
        assert!(waiting.try_recv().unwrap().is_ok());
        request.finish_link(Err("late completion".into()));
        assert!(waiting.try_recv().is_err());
        drop(runtime);
    }
}

#[test]
fn source_changes_cancellation_save_conflicts_and_dispatch_failure_never_launch_a_link() {
    for failure in ["typing", "cancel", "disk", "dispatch"] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.follow_location(&source_location()).unwrap();
        let (incoming, waiting) = reply();
        let cancelled = incoming.cancelled.clone();
        writer.attach_linked_editor_reply(Some(incoming.reply), Some(incoming.cancelled));
        let (_runtime, mut saves, mut launches) = connect(&mut writer);
        if failure == "disk" {
            std::fs::write(directory.path().join("scene.recite"), "external changes").unwrap();
        }
        writer.resume_linked_editor();
        match failure {
            "typing" => writer.change(|model| {
                model.set_draft("Keep newer typing.".into());
                Ok(())
            }),
            "cancel" => cancelled.store(true, atomic::Ordering::Release),
            "dispatch" => {
                launches.close();
            }
            _ => {}
        }
        support::finish_work(&mut writer, &mut saves).unwrap();
        assert!(launches.try_recv().is_err(), "{failure}");
        assert!(waiting.try_recv().unwrap().is_err(), "{failure}");
        assert!(!writer.work.busy());
        if failure == "typing" {
            assert_eq!(writer.session.model().draft(), "Keep newer typing.");
        }
    }
}

#[test]
fn dropping_an_unstarted_link_replies_once_and_does_not_strand_an_activation() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.follow_location(&source_location()).unwrap();
    let (incoming, waiting) = reply();
    writer.attach_linked_editor_reply(Some(incoming.reply), Some(incoming.cancelled));
    drop(writer);
    assert!(waiting.try_recv().unwrap().is_err());
    assert!(waiting.try_recv().is_err());
}

#[test]
fn cold_source_links_keep_recovered_source_drafts_on_the_explicit_repair_surface() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.change(|model| {
        model.select(recite_writer_model::View::Source)?;
        model.stage_repaired_source(":: incomplete\n? recovered".to_owned().into())
    });
    let before = writer.session.model().recovery();
    assert!(writer.follow_location(&source_location()).is_err());
    assert_eq!(writer.session.model().recovery(), before);
    assert!(writer.source_repair_pending());
    assert!(writer.external.waiting.is_none());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        recite_writer_model::FIXTURE
    );
}

#[test]
#[cfg(unix)]
fn process_start_completes_the_route_before_exit_and_worker_cancellation_prevents_spawn() {
    for cancelled_before_start in [false, true] {
        let directory = support::project(recite_writer_model::FIXTURE).unwrap();
        let mut writer = support::open(&directory).unwrap();
        writer.editor = Some(EditorCommand {
            program: "/bin/sh".into(),
            args: vec!["-c".into(), "sleep 0.2".into()],
        });
        writer.follow_location(&source_location()).unwrap();
        let (incoming, waiting) = reply();
        let cancelled = incoming.cancelled.clone();
        writer.attach_linked_editor_reply(Some(incoming.reply), Some(incoming.cancelled));
        let (_runtime, mut saves, mut launches) = connect(&mut writer);
        writer.resume_linked_editor();
        support::finish_work(&mut writer, &mut saves).unwrap();
        let request = launches.try_recv().unwrap();
        cancelled.store(cancelled_before_start, atomic::Ordering::Release);
        let started = request.start();
        if cancelled_before_start {
            assert!(started.is_err());
            assert!(waiting.try_recv().unwrap().is_err());
        } else {
            let handoff = started.unwrap();
            assert!(
                handoff.poll().is_none(),
                "reply is independent of editor exit"
            );
            assert!(waiting.try_recv().unwrap().is_ok());
            cancelled.store(true, atomic::Ordering::Release);
            request.finish_link(Err("too late".into()));
            assert!(waiting.try_recv().is_err());
        }
    }
}
