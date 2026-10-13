//! Prepared adapter choices; source construction and semantic checks stay outside counters.
#[path = "../../../../crates/recite-benchmarks/benches/support/session_fixture.rs"]
mod fixture;

use recite_adapter::{SessionDriver, StartRequest};
use recite_core::ChoiceId;
use recite_runtime::{DialogueEvent, DialogueSessionOptions, LocaleResolution};

const OPERATIONS: usize = 64;

pub(super) struct AdapterSources {
    pub(super) drivers: Vec<SessionDriver>,
    pub(super) choice: ChoiceId,
    pub(super) outputs: Vec<Vec<DialogueEvent>>,
    expected_events: Vec<DialogueEvent>,
    expected_snapshot: Vec<u8>,
}

pub(super) fn sources_400() -> AdapterSources {
    let (asset, choice) = must(fixture::load(false, 400));
    let start = || {
        let mut driver = SessionDriver::new();
        must(driver.start(
            StartRequest {
                asset: &asset,
                block_id: None,
                options: DialogueSessionOptions::new(),
            },
            &fixture::ready,
            LocaleResolution::new(),
        ));
        driver
    };
    let drivers = (0..OPERATIONS).map(|_| start()).collect();
    let mut expected = start();
    let expected_events =
        must(expected.select_choice(choice.clone(), &fixture::ready, LocaleResolution::new()));
    let expected_snapshot = must(expected.snapshot());
    AdapterSources {
        drivers,
        choice,
        outputs: Vec::with_capacity(OPERATIONS),
        expected_events,
        expected_snapshot,
    }
}

pub(super) fn choose(driver: &mut SessionDriver, choice: ChoiceId) -> Vec<DialogueEvent> {
    must(driver.select_choice(choice, &fixture::ready, LocaleResolution::new()))
}

pub(super) fn validate(workload: AdapterSources) {
    assert_eq!(workload.drivers.len(), OPERATIONS);
    assert_eq!(workload.outputs.len(), OPERATIONS);
    assert!(!workload.expected_events.is_empty());
    for (driver, output) in workload.drivers.iter().zip(&workload.outputs) {
        assert_eq!(output, &workload.expected_events);
        assert_eq!(must(driver.snapshot()), workload.expected_snapshot);
    }
}

fn must<T, E: std::fmt::Display>(value: Result<T, E>) -> T {
    value.unwrap_or_else(|error| panic!("validated adapter source workload failed: {error}"))
}
