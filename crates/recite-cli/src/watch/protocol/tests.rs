use std::io::{self, Read};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use recite_compiler::{
    BuildCandidate, BuildCheck, BuildControl, BuildEngine, BuildFailure, BuildGeneration,
    BuildInput, BuildPreparedHandle, BuildPublisher, BuildRequest, BuildTarget,
    FreshnessAssessment, PreparedPublishIdentity, PublishAbortReason, PublishFailure,
    PublishOutcome, SnapshotGeneration,
};
use serde_json::Value;

use super::super::build::status_without_freshness;
use super::super::control::{ControlMessage, ControlTransport};
use super::super::emitter::StopReasonDto;
use super::super::emitter::WatchProtocol;
use super::super::events::WatchState;
use super::super::wire_types::{BuildCompletedData, CancellationDto};
use super::attempt::run_attempt_with;
use super::{complete_attempt, stop_with_error};

#[test]
fn cancellation_acknowledgement_precedes_cancelled_build_completion() {
    let mut output = Vec::new();
    let mut protocol = WatchProtocol::new(&mut output, None);
    protocol
        .cancel_requested(CancellationDto::User)
        .expect("cancel record");
    protocol
        .build_completed(
            BuildCompletedData::from_diagnostics(0, &[], &[]).expect("completion record"),
        )
        .expect("completion record");

    let events = String::from_utf8(output)
        .expect("protocol output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .map(|record| record["event"].as_str().expect("event").to_owned())
        .collect::<Vec<_>>();
    assert_eq!(
        events,
        vec!["watch.cancel.requested", "watch.build.completed"]
    );
}

#[test]
fn notify_error_is_a_recoverable_typed_record() {
    let mut output = Vec::new();
    let mut protocol = WatchProtocol::new(&mut output, None);
    protocol.notify_error().expect("notify error record");
    let records = String::from_utf8(output)
        .expect("protocol output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    assert_eq!(records[0]["event"], "watch.notify.error");
    assert_eq!(records[0]["data"]["error"]["type"], "watcher");
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
        self.started.send(()).expect("build starts");
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

    fn commit(&mut self, prepared: Self::Prepared) -> PublishOutcome {
        self.commits += 1;
        PublishOutcome::Published {
            targets: prepared
                .identity
                .candidates()
                .iter()
                .map(|candidate| candidate.target().clone())
                .collect(),
        }
    }
}

#[test]
fn active_cancellation_acknowledges_before_one_cancelled_completion() {
    let (control_input, input) = mpsc::channel();
    let reader = GateReader {
        chunks: input,
        pending: Vec::new(),
    };
    let (transport, controls) = ControlTransport::spawn(reader, None);
    let (started_sender, started_receiver) = mpsc::channel();
    let sender = std::thread::spawn(move || {
        started_receiver.recv().expect("build starts");
        control_input
            .send(b"{\"version\":1,\"command\":\"watch\",\"action\":\"cancel\"}\n".to_vec())
            .expect("control reader remains live");
    });
    let request = BuildRequest::new(
        BuildGeneration::initial(),
        SnapshotGeneration::initial(),
        [BuildInput::saved_source(
            recite_core::DocumentKey::new("dialogue/main.recite").expect("input key"),
            ":: start default\n",
        )],
    )
    .expect("request");
    let mut state = WatchState::new(std::path::PathBuf::from("."));
    let mut output = Vec::new();
    let mut protocol = WatchProtocol::new(&mut output, None);
    let attempt = run_attempt_with(
        &mut state,
        &transport,
        &mut protocol,
        super::super::wire_types::BuildTriggerDto::Initial,
        move |state, _sink, control| {
            state.next_build_generation().expect("build generation");
            let mut engine = BlockingEngine {
                started: started_sender,
            };
            let mut publisher = CountingPublisher { commits: 0 };
            let result = state
                .coordinator
                .run(request, control, &mut engine, &mut publisher)
                .expect("cancellation is a terminal result");
            assert_eq!(
                result.status(),
                recite_compiler::BuildTerminalStatus::Cancelled
            );
            assert_eq!(publisher.commits, 0);
            Ok(status_without_freshness(result, Vec::new()))
        },
    )
    .expect("attempt starts");
    let mut control_open = true;
    let mut cancel_emitted = false;
    assert!(
        complete_attempt(
            attempt,
            &transport,
            &controls,
            &mut control_open,
            &mut protocol,
            &mut cancel_emitted,
        )
        .expect("cancellation acknowledgement")
    );
    protocol
        .stopped(StopReasonDto::Cancelled, None)
        .expect("stop record");
    sender.join().expect("control sender");

    let events = String::from_utf8(output)
        .expect("protocol output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    let names = events
        .iter()
        .map(|record| record["event"].as_str().expect("event"))
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "watch.build.started",
            "watch.cancel.requested",
            "watch.build.completed",
            "watch.stopped"
        ]
    );
    assert_eq!(events[2]["data"]["status"], "cancelled");
    assert_eq!(events[2]["data"]["outcome"]["type"], "cancelled");
    assert_eq!(
        events[2]["data"]["publication"],
        serde_json::json!({"type":"not_attempted","reason":"cancelled"})
    );
}

struct ErrorReader;

impl Read for ErrorReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "control read failed",
        ))
    }
}

struct SignalledErrorReader {
    started: Receiver<()>,
    dropped: Sender<()>,
}

impl Read for SignalledErrorReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.started.recv().expect("build start signal");
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "control read failed",
        ))
    }
}

impl Drop for SignalledErrorReader {
    fn drop(&mut self) {
        self.dropped.send(()).expect("build waits for reader drop");
    }
}

struct QuickEngine {
    started: Sender<()>,
    reader_dropped: Receiver<()>,
}

impl BuildEngine for QuickEngine {
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
        _control: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        self.started.send(()).expect("build starts");
        self.reader_dropped
            .recv()
            .expect("reader reports drop after stream error");
        Ok(vec![BuildCandidate::new(
            BuildTarget::new("compiled/dialogue.recitec").expect("target"),
            b"candidate".to_vec(),
        )])
    }
}

#[test]
fn control_stream_read_error_stops_with_typed_record() {
    let (_transport, controls) = ControlTransport::spawn(ErrorReader, None);
    let error = match controls.recv().expect("stream error") {
        ControlMessage::Stream(error) => error,
        message => panic!("unexpected control message: {message:?}"),
    };
    let mut output = Vec::new();
    let mut protocol = WatchProtocol::new(&mut output, None);
    protocol
        .started(std::path::Path::new("."))
        .expect("started");
    stop_with_error(
        &mut protocol,
        crate::error::CliError::Io(error),
        "control",
        None,
    )
    .expect("stopped");
    let records = String::from_utf8(output)
        .expect("protocol output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    assert_eq!(records[1]["event"], "watch.stopped");
    assert_eq!(records[1]["data"]["reason"]["type"], "fatal");
    assert_eq!(records[1]["data"]["error"]["category"], "io");
    assert_eq!(records[1]["data"]["error"]["code"], "io");
}

#[test]
fn active_control_stream_error_emits_completion_before_fatal_stop() {
    let (build_started, reader_started) = mpsc::channel();
    let (reader_dropped, build_wait) = mpsc::channel();
    let reader = SignalledErrorReader {
        started: reader_started,
        dropped: reader_dropped,
    };
    let (transport, controls) = ControlTransport::spawn(reader, None);
    let request = BuildRequest::new(
        BuildGeneration::initial(),
        SnapshotGeneration::initial(),
        [BuildInput::saved_source(
            recite_core::DocumentKey::new("dialogue/main.recite").expect("input key"),
            ":: start default\n",
        )],
    )
    .expect("request");
    let mut state = WatchState::new(std::path::PathBuf::from("."));
    let mut output = Vec::new();
    let mut protocol = WatchProtocol::new(&mut output, None);
    let attempt = run_attempt_with(
        &mut state,
        &transport,
        &mut protocol,
        super::super::wire_types::BuildTriggerDto::Initial,
        move |state, _sink, control| {
            state.next_build_generation().expect("build generation");
            let mut engine = QuickEngine {
                started: build_started,
                reader_dropped: build_wait,
            };
            let mut publisher = CountingPublisher { commits: 0 };
            let result = state
                .coordinator
                .run(request, control, &mut engine, &mut publisher)
                .expect("build result");
            assert_eq!(
                result.status(),
                recite_compiler::BuildTerminalStatus::Succeeded
            );
            Ok(status_without_freshness(result, Vec::new()))
        },
    )
    .expect("attempt starts");
    let mut control_open = true;
    let mut cancel_emitted = false;
    let error = complete_attempt(
        attempt,
        &transport,
        &controls,
        &mut control_open,
        &mut protocol,
        &mut cancel_emitted,
    )
    .expect_err("stream failure is fatal");
    stop_with_error(&mut protocol, error, "control", None).expect("stopped");

    let events = String::from_utf8(output)
        .expect("protocol output")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("record"))
        .collect::<Vec<_>>();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0]["event"], "watch.build.started");
    assert_eq!(events[1]["event"], "watch.build.completed");
    assert_eq!(events[2]["event"], "watch.stopped");
}
