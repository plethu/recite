use super::*;
use xilem::core::{DynMessage, MessageCtx, SendMessage, ViewPathTracker};

fn deliver<V: View<App, (), ViewCtx, Element = NoElement>>(
    view: &V,
    state: &mut V::ViewState,
    context: &mut ViewCtx,
    app: &mut App,
    output: SendMessage,
) {
    let output = output.downcast::<Output>().unwrap();
    let mut message = MessageCtx::new(
        std::mem::take(context.environment()),
        Vec::new(),
        DynMessage(output),
    );
    view.message(state, &mut message, (), app);
    *context.environment() = message.finish().0;
}

#[test]
fn creating_and_opening_a_project_use_terminal_worker_receipts_and_preserve_existing_files() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("Created project");
    let mut app = App::empty();
    let (sent, outputs) = mpsc::sync_channel(4);
    let mut context = ViewCtx::new(
        Arc::new(Results(sent)),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = worker();
    let (_, mut state) = view.build(&mut context, &mut app);
    app.creating = true;
    app.path = path.display().to_string();
    app.name = "Native workflow".into();
    app.open_selected();
    assert!(app.auxiliary);
    deliver(
        &view,
        &mut state,
        &mut context,
        &mut app,
        outputs
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
    );
    assert!(!app.auxiliary);
    assert!(app.opening.is_some());
    assert!(
        app.writer.is_none(),
        "creation alone must not install an unprepared editor"
    );
    deliver(
        &view,
        &mut state,
        &mut context,
        &mut app,
        outputs
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
    );
    assert!(!app.show_welcome);
    assert!(app.opening.is_none());
    let writer = app.writer.as_ref().unwrap();
    assert_eq!(writer.session.project().unwrap().root(), path);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: start default")
    );
    let original = std::fs::read_to_string(path.join("recite.project.toml")).unwrap();
    let source = writer.session.model().document().source_snapshot();
    app.name = "Do not replace".into();
    app.open_selected();
    deliver(
        &view,
        &mut state,
        &mut context,
        &mut app,
        outputs
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
    );
    assert!(!app.auxiliary && app.opening.is_none());
    assert!(!app.status.is_empty());
    assert_eq!(
        std::fs::read_to_string(path.join("recite.project.toml")).unwrap(),
        original
    );
    assert_eq!(
        app.writer
            .as_ref()
            .unwrap()
            .session
            .model()
            .document()
            .source(),
        source.as_ref()
    );
    view.teardown(&mut state, &mut context, ());
}

#[test]
fn an_observed_cancelled_load_retires_its_project_before_the_worker_is_reused() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::empty();
    let (sent, outputs) = mpsc::sync_channel(4);
    let mut context = ViewCtx::new(
        Arc::new(Results(sent)),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = worker();
    let (_, mut state) = view.build(&mut context, &mut app);
    app.path = directory.path().display().to_string();
    app.open_selected();
    let prepared = outputs
        .recv_timeout(std::time::Duration::from_secs(5))
        .unwrap();
    app.cancel_open();
    deliver(&view, &mut state, &mut context, &mut app, prepared);
    assert!(app.writer.is_none() && app.opening.is_none());
    assert!(app.status.contains("cancelled"));
    // Queueing another load tests retirement ordering on the same worker.
    app.open_selected();
    deliver(
        &view,
        &mut state,
        &mut context,
        &mut app,
        outputs
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap(),
    );
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
    view.teardown(&mut state, &mut context, ());
}

#[test]
fn unavailable_project_worker_keeps_the_open_document_and_reports_each_requested_action() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(open(&directory).unwrap());
    let before = app.writer.as_ref().unwrap().session.model().recovery();
    app.browse();
    assert!(!app.auxiliary && app.status.contains("unavailable"));
    app.path = directory.path().display().to_string();
    app.open_selected();
    assert!(app.opening.is_none() && app.status.contains("unavailable"));
    app.creating = true;
    app.name = "Preserved".into();
    app.open_selected();
    assert!(!app.auxiliary && app.status.contains("unavailable"));
    app.show_examples();
    assert!(app.opening.is_none());
    assert_eq!(
        app.writer.as_ref().unwrap().session.model().recovery(),
        before
    );
}
