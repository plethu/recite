use super::*;
use crate::tests::support::{open, project};
use std::sync::atomic;
use tokio::sync::mpsc as async_mpsc;
#[path = "workflow/tests.rs"]
mod workflow;

#[test]
#[cfg(target_os = "linux")]
fn cold_activation_returns_its_terminal_reply_and_retires_the_previous_owner() {
    let first = project(recite_writer_model::FIXTURE).unwrap();
    let next = project(recite_writer_model::FIXTURE).unwrap();
    let mut app = App::with_writer(open(&first).unwrap());
    let (sender, mut jobs) = async_mpsc::unbounded_channel();
    app.sender = Some(sender);
    let (reply, response) = mpsc::channel();
    app.accept_activation(recite_writer_session::activation::IncomingRoute {
        project: Some(next.path().to_owned()),
        route: None,
        document: None,
        editor: None,
        cancelled: Arc::new(atomic::AtomicBool::new(false)),
        reply,
    });
    assert!(response.try_recv().is_err());
    let Job::Open(request) = jobs.try_recv().unwrap() else {
        panic!("cold activation")
    };
    let result = load(&request);
    let (retire, retired) = mpsc::sync_channel(1);
    app.accept_open(Opened {
        request,
        result,
        retire,
    });
    assert!(response.try_recv().unwrap().is_ok());
    assert_eq!(
        app.writer
            .as_ref()
            .unwrap()
            .session
            .project()
            .unwrap()
            .root(),
        next.path()
    );
    let previous = retired.try_recv().unwrap().unwrap();
    assert_eq!(previous.session.project().unwrap().root(), first.path());
    drop(previous);
    assert!(recite_writer_session::ProjectSession::open(first.path(), None).is_ok());
}

#[test]
fn obsolete_cold_load_preserves_the_live_writer_and_retires_the_unused_owner() {
    for action in ["edit", "cancel", "close"] {
        let first = project(recite_writer_model::FIXTURE).unwrap();
        let second = project(recite_writer_model::FIXTURE).unwrap();
        let mut app = App::with_writer(open(&first).unwrap());
        let (sender, mut jobs) = async_mpsc::unbounded_channel();
        app.sender = Some(sender);
        app.start_open(
            LaunchOptions {
                intent: Startup::Writer {
                    project: Some(second.path().to_owned()),
                    route: None,
                },
                document: None,
                editor: None,
            },
            None,
            None,
        )
        .unwrap();
        let Job::Open(request) = jobs.try_recv().unwrap() else {
            panic!("cold load request")
        };
        let result = load(&request);
        assert!(result.is_ok());
        match action {
            "cancel" => app.cancel_open(),
            "close" => app.request_close(),
            _ => app.writer.as_mut().unwrap().change(|model| {
                model.set_draft("Keep this newer draft.".into());
                Ok(())
            }),
        }
        let (retire, retired) = mpsc::sync_channel(1);
        app.accept_open(Opened {
            request,
            result,
            retire,
        });
        let writer = app.writer.as_ref().unwrap();
        assert_eq!(writer.session.project().unwrap().root(), first.path());
        if action == "edit" {
            assert_eq!(writer.session.model().draft(), "Keep this newer draft.");
        }
        if action == "close" {
            assert!(writer.phase != Phase::Writing);
        }
        assert!(app.status.contains("current work remains open"));
        let unused = retired.try_recv().unwrap().unwrap();
        assert_eq!(unused.session.project().unwrap().root(), second.path());
        drop(unused);
        assert!(recite_writer_session::ProjectSession::open(second.path(), None).is_ok());
    }
}

#[derive(Debug)]
struct Results(mpsc::SyncSender<xilem::core::SendMessage>);
impl xilem::core::RawProxy for Results {
    fn send_message(
        &self,
        _path: Arc<[xilem::core::ViewId]>,
        message: xilem::core::SendMessage,
    ) -> Result<(), xilem::core::ProxyError> {
        self.0
            .send(message)
            .map_err(|error| xilem::core::ProxyError::DriverFinished(error.0))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

#[test]
#[cfg(target_os = "linux")]
fn idle_activation_polling_posts_no_ui_work_but_delivers_actual_launches() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (results, waiting) = mpsc::sync_channel(1);
    let proxy = MessageProxy::new(Arc::new(Results(results)), Arc::from([]));
    let (jobs, requests) = async_mpsc::unbounded_channel();
    let (routes, incoming) = mpsc::channel();
    jobs.send(Job::Listen(ActivationInbox::new(incoming)))
        .unwrap();
    let worker = runtime.spawn(serve(proxy, requests));
    assert!(
        matches!(
            waiting.recv_timeout(std::time::Duration::from_millis(120)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ),
        "idle activation must not rebuild the native writer"
    );
    let (reply, _response) = mpsc::channel();
    routes
        .send(recite_writer_session::activation::IncomingRoute {
            project: None,
            route: Some("recite://writer/write?scene=owned.recite".into()),
            document: Some("owned.recite".into()),
            editor: None,
            cancelled: Arc::new(atomic::AtomicBool::new(false)),
            reply,
        })
        .unwrap();
    let output = waiting
        .recv_timeout(std::time::Duration::from_secs(2))
        .unwrap()
        .downcast::<Output>()
        .unwrap();
    let Output::Activation(request) = *output else {
        panic!("forwarded launch")
    };
    assert_eq!(request.document, Some("owned.recite".into()));
    drop(jobs);
    runtime.block_on(worker).unwrap();
}
