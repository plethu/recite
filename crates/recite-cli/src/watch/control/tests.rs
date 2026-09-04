use std::io::{self, Cursor, Read};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use recite_compiler::{
    BuildCandidate, BuildCheck, BuildControl, BuildCoordinator, BuildEngine, BuildFailure,
    BuildGeneration, BuildInput, BuildPreparedHandle, BuildPublisher, BuildRequest, BuildTarget,
    BuildTerminalStatus, FreshnessAssessment, PreparedPublishIdentity, PublishAbortReason,
    PublishFailure, PublishOutcome, SnapshotGeneration,
};
use recite_core::DocumentKey;

use super::{ControlError, ControlMessage, ControlTransport, matching_invocation};

fn receive(input: &str, invocation_id: Option<&str>) -> ControlMessage {
    let (_transport, receiver) =
        ControlTransport::spawn(Cursor::new(input.as_bytes().to_vec()), invocation_id);
    receiver.recv().expect("control message")
}

#[test]
fn accepts_only_version_one_watch_cancel() {
    assert!(matches!(
        receive(r#"{"version":1,"command":"watch","action":"cancel"}"#, None),
        ControlMessage::Cancel
    ));
    assert!(matches!(
        receive(r#"{"version":2,"command":"watch","action":"cancel"}"#, None),
        ControlMessage::Error(ControlError::UnsupportedVersion)
    ));
    assert!(matches!(
        receive(
            r#"{"version":1,"command":"compile","action":"cancel"}"#,
            None
        ),
        ControlMessage::Error(ControlError::UnsupportedCommand)
    ));
    assert!(matches!(
        receive(r#"{"version":1,"command":"watch","action":"stop"}"#, None),
        ControlMessage::Error(ControlError::UnsupportedAction)
    ));
    assert!(matches!(
        receive("not-json", None),
        ControlMessage::Error(ControlError::Malformed)
    ));
}

#[test]
fn invocation_id_is_optional_but_must_match_when_present() {
    assert!(matching_invocation(Some("one"), None));
    assert!(matching_invocation(Some("one"), Some("one")));
    assert!(!matching_invocation(Some("one"), Some("two")));
    assert!(matching_invocation(None, None));
    assert!(!matching_invocation(None, Some("one")));
}

#[test]
fn valid_cancel_reaches_active_build_control() {
    let (transport, receiver) = ControlTransport::spawn(
        Cursor::new(b"{\"version\":1,\"command\":\"watch\",\"action\":\"cancel\"}\n".to_vec()),
        None,
    );
    let control = BuildControl::new();
    transport.begin_build(&control);
    assert!(matches!(
        receiver.recv().expect("control message"),
        ControlMessage::Cancel
    ));
    assert!(matches!(
        control.cancellation(),
        Some(recite_compiler::BuildCancellation::User)
    ));
}

struct GateReader {
    chunks: Receiver<Vec<u8>>,
    pending: Vec<u8>,
}

impl Read for GateReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.pending.is_empty() {
            self.pending = self.chunks.recv().unwrap_or_default();
        }
        let count = self.pending.len().min(buffer.len());
        buffer[..count].copy_from_slice(&self.pending[..count]);
        self.pending.drain(..count);
        Ok(count)
    }
}

struct BlockingEngine {
    started: Sender<()>,
}

#[allow(
    clippy::disallowed_methods,
    reason = "test watchdog bounds the injected cancellation seam"
)]
impl BuildEngine for BlockingEngine {
    fn check(&mut self, request: &BuildRequest, _control: &BuildControl) -> BuildCheck {
        BuildCheck::new(
            request,
            Vec::new(),
            FreshnessAssessment::fresh(request.fingerprints().clone()),
        )
    }

    fn build(
        &mut self,
        _request: &BuildRequest,
        control: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        self.started
            .send(())
            .expect("build gate receiver remains live");
        let started = Instant::now();
        while control.cancellation().is_none() {
            if started.elapsed() > Duration::from_secs(5) {
                return Err(BuildFailure::Engine {
                    reason: recite_compiler::BuildFailureReason::Host,
                });
            }
            std::thread::yield_now();
        }
        Ok(vec![BuildCandidate::new(
            BuildTarget::new("compiled/dialogue.recitec").expect("target"),
            b"candidate".to_vec(),
        )])
    }
}

struct PreparedBuild {
    identity: PreparedPublishIdentity,
}

impl BuildPreparedHandle for PreparedBuild {
    fn identity(&self) -> PreparedPublishIdentity {
        self.identity.clone()
    }
}

struct CountingPublisher {
    commits: usize,
}

impl BuildPublisher for CountingPublisher {
    type Prepared = PreparedBuild;

    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        _control: &BuildControl,
    ) -> Result<Self::Prepared, PublishFailure> {
        Ok(PreparedBuild {
            identity: PreparedPublishIdentity::for_request(request, candidates.to_vec()),
        })
    }

    fn abort(&mut self, _prepared: Option<Self::Prepared>, _reason: PublishAbortReason) {}

    fn commit(&mut self, _prepared: Self::Prepared) -> PublishOutcome {
        self.commits += 1;
        PublishOutcome::Published {
            targets: Vec::new(),
        }
    }
}

#[test]
fn active_transport_cancellation_cannot_publish_a_candidate() {
    let (control_input, input) = mpsc::channel();
    let reader = GateReader {
        chunks: input,
        pending: Vec::new(),
    };
    let (transport, controls) = ControlTransport::spawn(reader, None);
    let control = BuildControl::new();
    transport.begin_build(&control);

    let request = BuildRequest::new(
        BuildGeneration::new(1),
        SnapshotGeneration::new(1),
        [BuildInput::saved_source(
            DocumentKey::new("dialogue/main.recite").expect("input key"),
            ":: start default\n",
        )],
    )
    .expect("request");
    let (started_sender, started_receiver) = mpsc::channel();
    let sender = std::thread::spawn(move || {
        started_receiver.recv().expect("build starts");
        control_input
            .send(b"{\"version\":1,\"command\":\"watch\",\"action\":\"cancel\"}\n".to_vec())
            .expect("control reader remains live");
    });
    let mut engine = BlockingEngine {
        started: started_sender,
    };
    let mut publisher = CountingPublisher { commits: 0 };
    let result = BuildCoordinator::new()
        .run(request, &control, &mut engine, &mut publisher)
        .expect("cancellation is a terminal build result");
    sender.join().expect("control sender");

    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert!(matches!(
        controls.recv().expect("cancel control"),
        ControlMessage::Cancel
    ));
    assert_eq!(publisher.commits, 0);
    transport.end_build();
}
