use super::*;
use crate::tests::support::{self, click, labels, open, project};
use tokio::sync::mpsc as async_mpsc;

const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n";

fn begin(writer: &mut Writer) -> Result<Arc<Request>, Box<dyn std::error::Error>> {
    let (sender, mut requests) = async_mpsc::unbounded_channel();
    writer.trial.sender = Some(sender);
    writer.start_trial();
    let request = requests.try_recv()?;
    writer.start_trial();
    assert!(requests.try_recv().is_err(), "only one preparation may run");
    Ok(request)
}

fn finish(
    writer: &mut Writer,
    request: Arc<Request>,
) -> Result<Option<Retired>, Box<dyn std::error::Error>> {
    let result = request
        .prepare(&CancellationToken::new())
        .map_err(|error| error.to_string());
    let (receipt, returned) = mpsc::sync_channel(1);
    writer.accept_trial(Output {
        request,
        result,
        receipt,
    });
    assert!(!writer.trial.busy());
    Ok(returned.recv()?.1)
}

fn install(writer: &mut Writer) -> Result<(), Box<dyn std::error::Error>> {
    let request = begin(writer)?;
    assert!(finish(writer, request)?.is_none());
    Ok(())
}

#[test]
fn cancelled_or_obsolete_preparations_never_replace_the_running_trial() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    for invalidate in 0..5 {
        let request = begin(&mut writer).unwrap();
        match invalidate {
            0 => writer.trial.cancel(),
            1 => writer.trial.visible = false,
            2 => writer.phase = Phase::ConfirmClose,
            3 => writer.change(|model| {
                model.set_draft("Draft".into());
                Ok(())
            }),
            _ => {
                writer.reload();
                support::finish_navigation(&mut writer, &mut navigation).unwrap();
            }
        }
        assert!(matches!(
            finish(&mut writer, request).unwrap(),
            Some(Retired::Prepared(_))
        ));
        if invalidate != 4 {
            assert_eq!(
                writer.session.model().preview_page().unwrap().text,
                "Hello."
            );
        }
        writer.phase = Phase::Writing;
        writer.change(|model| {
            model.discard();
            Ok(())
        });
    }
}

#[test]
fn successful_restart_returns_the_old_run_to_the_worker_and_failure_preserves_it() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    let request = begin(&mut writer).unwrap();
    assert!(matches!(
        finish(&mut writer, request).unwrap(),
        Some(Retired::Preview(_))
    ));
    let request = begin(&mut writer).unwrap();
    let (receipt, returned) = mpsc::sync_channel(1);
    writer.accept_trial(Output {
        request,
        result: Err("Could not prepare".into()),
        receipt,
    });
    assert!(returned.recv().unwrap().1.is_none());
    assert_eq!(
        writer.trial.status,
        "Could not prepare\nThe running preview below is unchanged."
    );
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello."
    );
    writer.trial.sender = None;
    writer.start_trial();
    assert!(writer.trial.status.contains("unavailable"));
    let (sender, requests) = async_mpsc::unbounded_channel();
    drop(requests);
    writer.trial.sender = Some(sender);
    writer.start_trial();
    assert!(writer.trial.status.contains("stopped"));
    writer.change(|model| {
        model.set_draft("Unapplied".into());
        Ok(())
    });
    writer.start_trial();
    assert!(writer.trial.status.contains("Apply or discard"));
    assert!(!writer.trial.busy());
}

#[test]
fn native_preview_actions_play_choices_conditions_effects_and_keep_authoring_undo() {
    let source = ":: start default\n> line@11111111111111111111\n  Hello.\n? go@22222222222222222222\n  Choose the branch\n  -> branch\n:: branch\n:if trusts(player)\n  > yes@33333333333333333333\n    Yes.\n:else\n  > no@44444444444444444444\n    No.\n! blocking overlay(work)\n-> END\n";
    let directory = project(source).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    click(
        &mut writer,
        &["Continue preview", "Choose the branch", "Answer true"],
    )
    .unwrap();
    assert_eq!(writer.session.model().preview_page().unwrap().text, "Yes.");
    click(
        &mut writer,
        &["Continue preview", "Acknowledge effect", "Continue preview"],
    )
    .unwrap();
    assert!(writer.session.model().preview_page().unwrap().ended);
    assert!(!writer.session.dirty(), "playback is transient");
    writer.change(|model| {
        model.set_draft("Accepted.".into());
        model.apply()
    });
    assert!(writer.session.model().preview_stale());
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label.contains("older inputs"))
    );
    click(&mut writer, &["Hide preview", "Undo applied"]).unwrap();
    assert!(!writer.trial.visible);
    assert_eq!(writer.session.model().draft(), "Hello.");
}

#[test]
fn enum_condition_and_effect_failure_use_the_existing_runtime_contract() {
    let source = ":: start default\n:match mood(player)\n  :case calm\n    > calm@11111111111111111111\n      Calm.\n  :case _\n    > other@22222222222222222222\n      Other.\n! blocking overlay(work)\n-> END\n";
    let directory = project(source).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    writer.trial.answer = "calm".into();
    click(
        &mut writer,
        &[
            "Answer condition",
            "Continue preview",
            "Report effect failure",
        ],
    )
    .unwrap();
    assert!(!writer.trial.status.is_empty());
    assert!(!writer.session.dirty());
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
fn native_worker_waits_for_disposal_receipts_and_exits_when_its_owner_disconnects() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (results, waiting) = mpsc::sync_channel(1);
    let proxy = MessageProxy::new(Arc::new(Results(results)), Arc::from([]));
    let (sender, requests) = async_mpsc::unbounded_channel();
    writer.trial.sender = Some(sender);
    let worker = runtime.spawn(serve(proxy, requests));
    for _ in 0..20 {
        writer.start_trial();
        let output = waiting
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap()
            .downcast::<Output>()
            .unwrap();
        assert!(output.result.is_ok());
        writer.start_trial();
        assert!(
            waiting.try_recv().is_err(),
            "busy state bounds queued compilation"
        );
        writer.accept_trial(*output);
        assert_eq!(
            writer.session.model().preview_page().unwrap().text,
            "Hello."
        );
    }
    writer.trial.sender = None;
    runtime.block_on(worker).unwrap();
    let proxy = MessageProxy::new(Arc::new(support::NoProxy), Arc::from([]));
    let (sender, requests) = async_mpsc::unbounded_channel();
    sender
        .send(Arc::new(
            Request::capture(
                writer.session.model(),
                recite_writer_session::preview::Inputs::default(),
                None,
            )
            .unwrap(),
        ))
        .unwrap();
    drop(sender);
    runtime.block_on(serve(proxy, requests));
}

#[path = "tests/bounds/tests.rs"]
mod bounds;

#[path = "tests/inputs/tests.rs"]
mod inputs;
