use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildAuthorityFence, BuildCandidate, BuildCheck, BuildControl,
    BuildCoordinator, BuildEngine, BuildFailure, BuildInput, BuildRequest, BuildTerminalStatus,
};
use std::collections::BTreeMap;
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

struct BlockingEngine {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    candidates: Vec<BuildCandidate>,
}
impl BuildEngine for BlockingEngine {
    fn check(&mut self, request: &BuildRequest, _: &BuildControl) -> BuildCheck {
        self.entered.wait();
        self.release.wait();
        BuildCheck::passed(request)
    }
    fn build(
        &mut self,
        _: &BuildRequest,
        _: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        Ok(self.candidates.clone())
    }
}

#[test]
fn external_supersession_before_a_permit_allows_only_b_to_publish() {
    let request_a = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let request_b = make_request(2, [BuildInput::saved_source(key("a.recite"), "b")]);
    let fence = BuildAuthorityFence::new(BuildAuthority::from_request(&request_a));
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let control_a = BuildControl::new();
    let worker = {
        let fence = fence.clone();
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        let control = control_a.clone();
        let request = request_a.clone();
        thread::spawn(move || {
            let mut engine = BlockingEngine {
                entered,
                release,
                candidates: vec![candidate("dialogue.recitec", b"A")],
            };
            let mut publisher = FakePublisher::new();
            let mut coordinator = BuildCoordinator::with_fence(fence);
            let result = coordinator
                .run(request, &control, &mut engine, &mut publisher)
                .unwrap_or_else(|error| panic!("blocked A transitions: {error}"));
            (result, publisher)
        })
    };
    entered.wait();
    control_a.supersede(recite_compiler::BuildGeneration::new(2));
    let mut engine_b = FakeEngine::new([candidate("dialogue.recitec", b"B")]);
    let mut publisher_b = FakePublisher::new();
    let result_b = BuildCoordinator::with_fence(fence)
        .run(
            request_b,
            &BuildControl::new(),
            &mut engine_b,
            &mut publisher_b,
        )
        .unwrap_or_else(|error| panic!("B transitions: {error}"));
    assert_eq!(result_b.status(), BuildTerminalStatus::Succeeded);
    assert_eq!(
        publisher_b.published.get("dialogue.recitec"),
        Some(&b"B".to_vec())
    );
    release.wait();
    let (result_a, publisher_a) = worker
        .join()
        .unwrap_or_else(|_| panic!("A worker completes"));
    assert_eq!(result_a.status(), BuildTerminalStatus::Superseded);
    assert_eq!(publisher_a.commit_calls, 0);
    assert!(publisher_a.published.is_empty());
}

struct BlockingBuildEngine {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    candidates: Vec<BuildCandidate>,
}
impl BuildEngine for BlockingBuildEngine {
    fn check(&mut self, request: &BuildRequest, _: &BuildControl) -> BuildCheck {
        BuildCheck::passed(request)
    }
    fn build(
        &mut self,
        _: &BuildRequest,
        _: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        self.entered.wait();
        self.release.wait();
        Ok(self.candidates.clone())
    }
}

#[test]
fn external_supersession_during_build_prevents_old_publication() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let control = BuildControl::new();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker_control = control.clone();
    let worker_entered = Arc::clone(&entered);
    let worker_release = Arc::clone(&release);
    let worker = thread::spawn(move || {
        let mut engine = BlockingBuildEngine {
            entered: worker_entered,
            release: worker_release,
            candidates: vec![candidate("dialogue.recitec", b"A")],
        };
        let mut publisher = FakePublisher::new();
        let result = run(request, &worker_control, &mut engine, &mut publisher);
        (result, publisher)
    });
    entered.wait();
    control.supersede(recite_compiler::BuildGeneration::new(2));
    release.wait();
    let (result, publisher) = worker
        .join()
        .unwrap_or_else(|_| panic!("blocked build worker completes"));
    assert_eq!(result.status(), BuildTerminalStatus::Superseded);
    assert_eq!(publisher.commit_calls, 0);
    assert!(publisher.published.is_empty());
}

struct BlockingPreparePublisher {
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    commit_calls: usize,
}
impl recite_compiler::BuildPublisher for BlockingPreparePublisher {
    type Prepared = super::support::FakePrepared;
    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        _: &BuildControl,
    ) -> Result<Self::Prepared, recite_compiler::PublishFailure> {
        self.entered.wait();
        self.release.wait();
        Ok(super::support::FakePrepared {
            identity: recite_compiler::PreparedPublishIdentity::for_request(
                request,
                candidates.to_vec(),
            ),
            candidates: candidates.to_vec(),
        })
    }
    fn abort(&mut self, _: Option<Self::Prepared>, _: recite_compiler::PublishAbortReason) {}
    fn commit(&mut self, _: Self::Prepared) -> recite_compiler::PublishOutcome {
        self.commit_calls += 1;
        recite_compiler::PublishOutcome::Published {
            targets: Vec::new(),
        }
    }
}

#[test]
fn external_cancellation_during_prepare_aborts_without_commit() {
    let request = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let control = BuildControl::new();
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker_control = control.clone();
    let worker_entered = Arc::clone(&entered);
    let worker_release = Arc::clone(&release);
    let worker = thread::spawn(move || {
        let mut engine = FakeEngine::new([candidate("dialogue.recitec", b"A")]);
        let mut publisher = BlockingPreparePublisher {
            entered: worker_entered,
            release: worker_release,
            commit_calls: 0,
        };
        let result = run(request, &worker_control, &mut engine, &mut publisher);
        (result, publisher)
    });
    entered.wait();
    control.cancel();
    release.wait();
    let (result, publisher) = worker
        .join()
        .unwrap_or_else(|_| panic!("blocked prepare worker completes"));
    assert_eq!(result.status(), BuildTerminalStatus::Cancelled);
    assert_eq!(publisher.commit_calls, 0);
}

struct BlockingCommitPublisher {
    shared: Arc<Mutex<BTreeMap<String, Vec<u8>>>>,
    entered: Arc<Barrier>,
    release: Arc<Barrier>,
    bytes: Vec<u8>,
    block: bool,
}
struct BlockingPrepared {
    identity: recite_compiler::PreparedPublishIdentity,
    candidates: Vec<BuildCandidate>,
}
impl recite_compiler::BuildPreparedHandle for BlockingPrepared {
    fn identity(&self) -> recite_compiler::PreparedPublishIdentity {
        self.identity.clone()
    }
}
impl recite_compiler::BuildPublisher for BlockingCommitPublisher {
    type Prepared = BlockingPrepared;
    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        _: &BuildControl,
    ) -> Result<Self::Prepared, recite_compiler::PublishFailure> {
        Ok(BlockingPrepared {
            identity: recite_compiler::PreparedPublishIdentity::for_request(
                request,
                candidates.to_vec(),
            ),
            candidates: candidates.to_vec(),
        })
    }
    fn abort(&mut self, _: Option<Self::Prepared>, _: recite_compiler::PublishAbortReason) {}
    fn commit(&mut self, prepared: Self::Prepared) -> recite_compiler::PublishOutcome {
        if self.block {
            self.entered.wait();
            self.release.wait();
        }
        let candidate = prepared.candidates.first();
        if let Some(candidate) = candidate {
            self.shared
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .insert(candidate.target().as_str().to_owned(), self.bytes.clone());
        }
        recite_compiler::PublishOutcome::Published {
            targets: prepared
                .candidates
                .iter()
                .map(|candidate| candidate.target().clone())
                .collect(),
        }
    }
}

#[test]
fn permit_commit_linearizes_before_newer_install_and_newer_bytes_win() {
    let request_a = make_request(1, [BuildInput::saved_source(key("a.recite"), "a")]);
    let request_b = make_request(2, [BuildInput::saved_source(key("a.recite"), "b")]);
    let fence = BuildAuthorityFence::new(BuildAuthority::from_request(&request_a));
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let shared = Arc::new(Mutex::new(BTreeMap::new()));
    let worker = {
        let fence = fence.clone();
        let entered = Arc::clone(&entered);
        let release = Arc::clone(&release);
        let shared = Arc::clone(&shared);
        let request = request_a.clone();
        thread::spawn(move || {
            let mut engine = FakeEngine::new([candidate("dialogue.recitec", b"A")]);
            let mut publisher = BlockingCommitPublisher {
                shared,
                entered,
                release,
                bytes: b"A".to_vec(),
                block: true,
            };
            BuildCoordinator::with_fence(fence)
                .run(request, &BuildControl::new(), &mut engine, &mut publisher)
                .unwrap_or_else(|error| panic!("A commit transitions: {error}"))
                .status()
        })
    };
    entered.wait();
    let shared_b = Arc::clone(&shared);
    let fence_b = fence.clone();
    let b_thread = thread::spawn(move || {
        let mut engine = FakeEngine::new([candidate("dialogue.recitec", b"B")]);
        let mut publisher = BlockingCommitPublisher {
            shared: shared_b,
            entered: Arc::new(Barrier::new(1)),
            release: Arc::new(Barrier::new(1)),
            bytes: b"B".to_vec(),
            block: false,
        };
        BuildCoordinator::with_fence(fence_b)
            .run(request_b, &BuildControl::new(), &mut engine, &mut publisher)
            .unwrap_or_else(|error| panic!("B commit transitions: {error}"))
            .status()
    });
    release.wait();
    assert_eq!(
        worker.join().unwrap_or_else(|_| panic!("A completes")),
        BuildTerminalStatus::Succeeded
    );
    assert_eq!(
        b_thread.join().unwrap_or_else(|_| panic!("B completes")),
        BuildTerminalStatus::Succeeded
    );
    assert_eq!(
        shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get("dialogue.recitec"),
        Some(&b"B".to_vec())
    );
}
