use super::*;
use crate::tests::support;
use recite_writer_session::{activation::IncomingRoute, startup::EditorCommand};
use tokio::sync::mpsc as async_mpsc;
use xilem::core::View as _;

fn request(
    project: PathBuf,
    route: Option<&str>,
) -> (IncomingRoute, mpsc::Receiver<Result<(), String>>) {
    let (reply, response) = mpsc::channel();
    (
        IncomingRoute {
            project: Some(project),
            route: route.map(str::to_owned),
            document: None,
            editor: None,
            cancelled: Arc::new(atomic::AtomicBool::new(false)),
            reply,
        },
        response,
    )
}

#[test]
fn forwarded_link_can_retry_after_the_native_navigation_worker_starts() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    std::fs::write(&other, ":: other default\n> @m3ffvrt\n  Other.\n-> END\n").unwrap();
    let app = App::with_writer(support::open(&directory).unwrap());
    let before = app.writer.as_ref().unwrap().session.model().recovery();
    let view = crate::navigation::worker().map_state(|app: &mut App| app.writer.as_mut().unwrap());
    let mut worker = support::worker::Worker::new(app, view);
    let route = "recite://writer/write?scene=other.recite";
    let (incoming, response) = request(directory.path().to_owned(), Some(route));
    worker.state.accept_activation(incoming);
    assert_eq!(
        response.try_recv().unwrap(),
        Err("The navigation worker is starting. Retry this action in a moment.".into())
    );
    let writer = worker.state.writer.as_ref().unwrap();
    assert_eq!(writer.session.model().recovery(), before);
    assert!(!writer.navigation.busy());
    assert!(writer.navigation.history.step(false).is_none());
    worker.next();
    let (incoming, response) = request(directory.path().to_owned(), Some(route));
    worker.state.accept_activation(incoming);
    assert!(
        response.try_recv().is_err(),
        "wait for the checked navigation receipt"
    );
    while worker.state.writer.as_ref().unwrap().navigation.busy() {
        worker.next();
    }
    assert_eq!(response.try_recv().unwrap(), Ok(()));
    assert_eq!(
        worker
            .state
            .writer
            .as_ref()
            .unwrap()
            .session
            .project()
            .unwrap()
            .current()
            .path(),
        other
    );
    drop(worker.close());
}

#[test]
fn a_stopped_navigation_worker_is_not_reported_as_starting() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    let before = app.writer.as_ref().unwrap().session.model().recovery();
    let (sender, jobs) = async_mpsc::unbounded_channel();
    app.writer.as_mut().unwrap().navigation.sender = Some(sender);
    drop(jobs);
    let (incoming, response) = request(
        directory.path().to_owned(),
        Some("recite://writer/write?scene=scene.recite"),
    );
    app.accept_activation(incoming);
    assert_eq!(
        response.try_recv().unwrap(),
        Err("The navigation worker is unavailable. Reopen Writer to retry.".into())
    );
    assert_eq!(
        app.writer.as_ref().unwrap().session.model().recovery(),
        before
    );
}

#[test]
fn rejected_activation_replies_without_changing_the_manuscript_or_editor() {
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.writer.as_mut().unwrap().change(|model| {
        model.set_draft("Keep pending manuscript.".into());
        Ok(())
    });
    let before = app.writer.as_ref().unwrap().session.model().recovery();
    for case in ["cancelled", "busy", "malformed"] {
        let (request, response) =
            request(directory.path().to_owned(), Some("recite://writer/unknown"));
        if case == "cancelled" {
            request.cancelled.store(true, atomic::Ordering::Release);
        }
        if case == "busy" {
            app.writer.as_mut().unwrap().phase = Phase::ConfirmClose;
        }
        app.accept_activation(request);
        assert!(response.try_recv().unwrap().is_err(), "{case}");
        assert!(!app.status.is_empty());
        assert_eq!(
            app.writer.as_ref().unwrap().session.model().recovery(),
            before
        );
        assert!(app.writer.as_ref().unwrap().editor.is_none());
        app.writer.as_mut().unwrap().phase = Phase::Writing;
    }
}

#[test]
fn same_project_launch_updates_literal_editor_options_and_checked_navigation_replies_at_completion()
{
    let directory = support::project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(support::open(&directory).unwrap());
    app.show_welcome = true;
    app.settings_open = true;
    let editor = EditorCommand {
        program: "my editor".into(),
        args: vec!["--literal=$HOME".into()],
    };
    let (mut incoming, response) = request(directory.path().to_owned(), None);
    incoming.editor = Some(editor.clone());
    app.accept_activation(incoming);
    assert!(response.try_recv().unwrap().is_ok());
    assert!(!app.show_welcome && !app.settings_open);
    assert_eq!(app.writer.as_ref().unwrap().editor, Some(editor));
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    let writer = app.writer.as_mut().unwrap();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    let (incoming, response) = request(
        directory.path().to_owned(),
        Some("recite://writer/write?scene=scene.recite"),
    );
    app.accept_activation(incoming);
    assert!(
        response.try_recv().is_err(),
        "acceptance must wait for the navigation receipt"
    );
    support::finish_navigation(app.writer.as_mut().unwrap(), &mut jobs).unwrap();
    assert!(response.try_recv().unwrap().is_ok());
    assert_eq!(
        app.writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        recite_writer_model::FIXTURE
    );
}
