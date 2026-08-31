use recite_benchmarks::preview::PreviewProject;
use recite_benchmarks::{BenchmarkFixture, BenchmarkScale};

const REPRESENTATIVE_FIXTURES: [BenchmarkFixture; 2] = [
    BenchmarkFixture::Synthetic(BenchmarkScale::Tiny),
    BenchmarkFixture::RealisticV1Pack,
];

#[test]
fn preview_traversal_is_deterministic_for_representative_fixtures()
-> Result<(), Box<dyn std::error::Error>> {
    for fixture in REPRESENTATIVE_FIXTURES {
        let project = PreviewProject::load(fixture)?;
        let mut first = project.start()?;
        let mut second = project.start()?;
        let first_events = project.collect_to_end(&mut first)?;
        let second_events = project.collect_to_end(&mut second)?;

        assert!(
            !first_events.is_empty(),
            "{fixture} preview produced no events"
        );
        assert_eq!(first_events, second_events, "{fixture} preview diverged");
    }
    Ok(())
}

#[test]
fn preview_retention_report_is_stable_and_structured() -> Result<(), Box<dyn std::error::Error>> {
    for fixture in REPRESENTATIVE_FIXTURES {
        let project = PreviewProject::load(fixture)?;
        let preview = project.at_first_prompt()?;
        let first = project.retention_report(&preview)?;
        let second = project.retention_report(&preview)?;

        assert_eq!(first, second, "{fixture} retention report changed");
        assert_eq!(first.fixture, fixture.as_str());
        assert!(first.snapshot.encoded_bytes > 0);
        assert!(first.trace.event_count > 0);
        assert!(first.trace.retained_bytes_lower_bound > 0);
        assert!(first.transcript_events > 0);
    }
    Ok(())
}

#[test]
fn preview_snapshot_restore_keeps_future_traversal_in_parity()
-> Result<(), Box<dyn std::error::Error>> {
    for fixture in REPRESENTATIVE_FIXTURES {
        let project = PreviewProject::load(fixture)?;
        let parity = project.restore_parity()?;

        assert!(parity.events_match, "{fixture} restored preview diverged");
        assert_eq!(parity.original_event_count, parity.restored_event_count);
        assert!(parity.original_event_count > 0);
    }
    Ok(())
}

#[test]
fn preview_defaults_pair_tiny_with_realistic_v1_pack() {
    assert_eq!(BenchmarkFixture::PREVIEW_DEFAULT, REPRESENTATIVE_FIXTURES);
}
