use super::*;
use crate::tests::support;
use recite_writer_session::{activation::IncomingRoute, startup::EditorCommand};
use tokio::sync::mpsc as async_mpsc;

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
