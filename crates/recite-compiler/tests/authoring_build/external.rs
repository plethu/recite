use std::sync::{Arc, Barrier};
use std::thread;

use super::support::*;
use recite_compiler::{
    BuildAuthority, BuildCandidate, BuildCheck, BuildControl, BuildCoordinator, BuildEngine,
    BuildFailure, BuildInput, BuildRequest, BuildState, BuildTerminalStatus,
};

#[test]
fn external_supersession_wins_while_a_is_blocked_and_only_b_publishes() {
    struct BlockingEngine {
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
        candidates: Vec<BuildCandidate>,
    }
    impl BuildEngine for BlockingEngine {
        fn check(&mut self, request: &BuildRequest, _control: &BuildControl) -> BuildCheck {
            self.entered.wait();
            self.release.wait();
            BuildCheck::passed(freshness(request))
        }

        fn build(
            &mut self,
            _request: &BuildRequest,
            _control: &BuildControl,
        ) -> Result<Vec<BuildCandidate>, BuildFailure> {
            Ok(self.candidates.clone())
        }
    }

    let request_a = make_request(
        1,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: a\n")],
    );
    let request_b = make_request(
        2,
        [BuildInput::saved_source(key("dialogue/a.recite"), ":: b\n")],
    );
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let control_a = BuildControl::new();
    let external_control = control_a.clone();
    let entered_worker = Arc::clone(&entered);
    let release_worker = Arc::clone(&release);
    let request_for_worker = request_a.clone();
    let worker = thread::spawn(move || {
        let mut engine = BlockingEngine {
            entered: entered_worker,
            release: release_worker,
            candidates: vec![candidate("dialogue.recitec", b"A")],
        };
        let mut publisher = FakePublisher::new();
        let mut coordinator = BuildCoordinator::new();
        let authority = BuildAuthority::from_request(&request_for_worker);
        let result = coordinator
            .run(
                request_for_worker,
                &control_a,
                &authority,
                &mut engine,
                &mut publisher,
            )
            .unwrap_or_else(|error| panic!("blocked build worker should transition: {error}"));
        (result, publisher, coordinator)
    });

    entered.wait();
    external_control.cancel();
    external_control.supersede(request_b.generation());
    release.wait();
    let (result_a, mut publisher, mut coordinator) = worker
        .join()
        .unwrap_or_else(|_| panic!("blocked build worker should complete"));
    assert_eq!(result_a.status(), BuildTerminalStatus::Superseded);
    assert_eq!(publisher.commit_calls, 0);
    assert!(publisher.published.is_empty());

    let control_b = BuildControl::new();
    let mut engine_b = FakeEngine::new([candidate("dialogue.recitec", b"B")]);
    let authority_b = BuildAuthority::from_request(&request_b);
    let result_b = coordinator
        .run(
            request_b,
            &control_b,
            &authority_b,
            &mut engine_b,
            &mut publisher,
        )
        .unwrap_or_else(|error| panic!("newer build should transition: {error}"));
    assert_eq!(result_b.status(), BuildTerminalStatus::Succeeded);
    assert!(matches!(coordinator.state(), BuildState::Succeeded { .. }));
    assert_eq!(publisher.commit_calls, 1);
    assert_eq!(
        publisher.published.get("dialogue.recitec"),
        Some(&b"B".to_vec())
    );
}
