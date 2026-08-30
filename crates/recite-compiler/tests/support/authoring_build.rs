use std::collections::BTreeMap;

use recite_compiler::{
    BuildCandidate, BuildCheck, BuildControl, BuildCoordinator, BuildEngine, BuildFailure,
    BuildGeneration, BuildInput, BuildPublisher, BuildRequest, BuildTarget, FreshnessAssessment,
    PreparedPublish, PublishAbortReason, PublishFailure, PublishFailureReason, PublishOutcome,
    SnapshotGeneration,
};
use recite_core::DocumentKey;

pub(crate) fn key(value: &str) -> DocumentKey {
    DocumentKey::new(value).unwrap_or_else(|error| panic!("test key is valid: {error}"))
}
pub(crate) fn target(value: &str) -> BuildTarget {
    BuildTarget::new(value).unwrap_or_else(|error| panic!("test target is valid: {error}"))
}
pub(crate) fn candidate(value: &str, bytes: &[u8]) -> BuildCandidate {
    BuildCandidate::new(target(value), bytes.to_vec())
}
pub(crate) fn make_request(
    generation: u64,
    inputs: impl IntoIterator<Item = BuildInput>,
) -> BuildRequest {
    BuildRequest::new(
        BuildGeneration::new(generation),
        SnapshotGeneration::new(generation),
        inputs,
    )
    .unwrap_or_else(|error| panic!("test request is valid: {error}"))
}
pub(crate) fn freshness(request: &BuildRequest) -> FreshnessAssessment {
    FreshnessAssessment::fresh(request.fingerprints().clone())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EngineCancellation {
    None,
    DuringCheck,
    DuringBuild,
}

pub(crate) struct FakeEngine {
    pub(crate) candidates: Vec<BuildCandidate>,
    pub(crate) cancellation: EngineCancellation,
    pub(crate) check_calls: usize,
    pub(crate) build_calls: usize,
}
impl FakeEngine {
    pub(crate) fn new(candidates: impl IntoIterator<Item = BuildCandidate>) -> Self {
        Self {
            candidates: candidates.into_iter().collect(),
            cancellation: EngineCancellation::None,
            check_calls: 0,
            build_calls: 0,
        }
    }
}
impl BuildEngine for FakeEngine {
    fn check(&mut self, request: &BuildRequest, control: &BuildControl) -> BuildCheck {
        self.check_calls += 1;
        if self.cancellation == EngineCancellation::DuringCheck {
            control.cancel();
        }
        BuildCheck::passed(request)
    }
    fn build(
        &mut self,
        _request: &BuildRequest,
        control: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure> {
        self.build_calls += 1;
        match self.cancellation {
            EngineCancellation::DuringBuild => control.cancel(),
            EngineCancellation::None | EngineCancellation::DuringCheck => {}
        }
        Ok(self.candidates.clone())
    }
}

pub(crate) struct FakePublisher {
    pub(crate) staged: Vec<BuildTarget>,
    pub(crate) published: BTreeMap<String, Vec<u8>>,
    pub(crate) prepare_calls: usize,
    pub(crate) commit_calls: usize,
    pub(crate) abort_calls: usize,
    pub(crate) cancel_after_prepare: Option<usize>,
    pub(crate) fail_target: Option<BuildTarget>,
    pub(crate) commit_outcome: Option<PublishOutcome>,
}
impl FakePublisher {
    pub(crate) fn new() -> Self {
        Self {
            staged: Vec::new(),
            published: BTreeMap::new(),
            prepare_calls: 0,
            commit_calls: 0,
            abort_calls: 0,
            cancel_after_prepare: None,
            fail_target: None,
            commit_outcome: None,
        }
    }
}
impl BuildPublisher for FakePublisher {
    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        control: &BuildControl,
    ) -> Result<PreparedPublish, PublishFailure> {
        for candidate in candidates {
            self.prepare_calls += 1;
            if self
                .fail_target
                .as_ref()
                .is_some_and(|failed| failed == candidate.target())
            {
                return Err(PublishFailure::Preparation {
                    target: candidate.target().clone(),
                    reason: PublishFailureReason::Storage,
                });
            }
            self.staged.push(candidate.target().clone());
            if self.cancel_after_prepare == Some(self.prepare_calls) {
                control.cancel();
            }
        }
        Ok(PreparedPublish::new(request, candidates.to_vec()))
    }
    fn abort(&mut self, _prepared: Option<PreparedPublish>, _reason: PublishAbortReason) {
        self.abort_calls += 1;
        self.staged.clear();
    }
    fn commit(&mut self, prepared: PreparedPublish) -> PublishOutcome {
        self.commit_calls += 1;
        let outcome = match self.commit_outcome.take() {
            Some(outcome) => outcome,
            None => PublishOutcome::Published {
                targets: prepared
                    .candidates()
                    .iter()
                    .map(|candidate| candidate.target().clone())
                    .collect(),
            },
        };
        match &outcome {
            PublishOutcome::Published { targets } => {
                for candidate in prepared
                    .candidates()
                    .iter()
                    .filter(|candidate| targets.contains(candidate.target()))
                {
                    self.published.insert(
                        candidate.target().as_str().to_owned(),
                        candidate.bytes().to_vec(),
                    );
                }
            }
            PublishOutcome::Partial { committed, .. } => {
                for candidate in prepared
                    .candidates()
                    .iter()
                    .filter(|candidate| committed.contains(candidate.target()))
                {
                    self.published.insert(
                        candidate.target().as_str().to_owned(),
                        candidate.bytes().to_vec(),
                    );
                }
            }
            PublishOutcome::NotAttempted { .. } | PublishOutcome::Refused { .. } => {}
            _ => {}
        }
        self.staged.clear();
        outcome
    }
}

pub(crate) fn run<E: BuildEngine, P: BuildPublisher>(
    request: BuildRequest,
    control: &BuildControl,
    engine: &mut E,
    publisher: &mut P,
) -> recite_compiler::BuildResult {
    BuildCoordinator::new()
        .run(request, control, engine, publisher)
        .unwrap_or_else(|error| panic!("test coordinator transition is valid: {error}"))
}
