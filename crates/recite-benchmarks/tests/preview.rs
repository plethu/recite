use recite_benchmarks::preview::{
    PreviewProject, PreviewRestoreParity, PreviewSnapshotShape, PreviewTraceShape,
    PreviewTraversalShape,
};
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
        let first_summary = project.traversal_summary(&mut first)?;
        let second_summary = project.traversal_summary(&mut second)?;

        assert!(
            first_summary.event_count > 0,
            "{fixture} preview produced no events"
        );
        assert_eq!(first_summary, second_summary, "{fixture} preview diverged");
    }
    Ok(())
}

#[test]
fn preview_evidence_report_is_stable_and_structured() -> Result<(), Box<dyn std::error::Error>> {
    for fixture in REPRESENTATIVE_FIXTURES {
        let project = PreviewProject::load(fixture)?;
        let first = project.evidence_report()?;
        let second = project.evidence_report()?;

        assert_eq!(first, second, "{fixture} retention report changed");
        assert_eq!(first.fixture, fixture.as_str());
        assert_eq!(first.retention.fixture, fixture.as_str());
        let (traversal, snapshot, trace, transcript_events, restore_events) = match fixture {
            BenchmarkFixture::Synthetic(BenchmarkScale::Tiny) => (
                PreviewTraversalShape {
                    event_count: 101,
                    event_hash: "62bbe805cb7fcb30d70827a03e2045175e1235c986542377a3b649959919d852"
                        .to_owned(),
                },
                PreviewSnapshotShape {
                    encoded_bytes: 3717,
                    selected_choice_count: 1,
                    deferred_effect_count: 1,
                },
                PreviewTraceShape {
                    event_count: 101,
                    condition_request_count: 16,
                    condition_result_count: 16,
                    line_count: 50,
                    prompt_count: 5,
                    choice_accepted_count: 2,
                    choice_selected_count: 5,
                    effect_count: 3,
                    immediate_effect_count: 2,
                    blocking_effect_count: 1,
                    deferred_effect_count: 2,
                    end_count: 1,
                    nested_slot_count: 225,
                    non_empty_collection_count: 138,
                    localized_lookup_count: 75,
                    plural_line_count: 0,
                },
                67,
                86,
            ),
            BenchmarkFixture::RealisticV1Pack => (
                PreviewTraversalShape {
                    event_count: 44,
                    event_hash: "c5de4929a1cb36ce6db6dca1c8f0aff37e678f58a51d0c8f5939efa1477ada20"
                        .to_owned(),
                },
                PreviewSnapshotShape {
                    encoded_bytes: 3452,
                    selected_choice_count: 1,
                    deferred_effect_count: 0,
                },
                PreviewTraceShape {
                    event_count: 44,
                    condition_request_count: 7,
                    condition_result_count: 7,
                    line_count: 9,
                    prompt_count: 5,
                    choice_accepted_count: 2,
                    choice_selected_count: 5,
                    effect_count: 4,
                    immediate_effect_count: 2,
                    blocking_effect_count: 2,
                    deferred_effect_count: 2,
                    end_count: 1,
                    nested_slot_count: 155,
                    non_empty_collection_count: 76,
                    localized_lookup_count: 26,
                    plural_line_count: 0,
                },
                28,
                36,
            ),
            _ => unreachable!("fixture is part of the representative set"),
        };
        assert_eq!(first.traversal, traversal);
        assert_eq!(first.retention.snapshot, snapshot);
        assert_eq!(first.retention.trace, trace);
        assert_eq!(first.retention.transcript_events, transcript_events);
        assert_eq!(
            first.restore,
            PreviewRestoreParity {
                events_match: true,
                original_event_count: restore_events,
                restored_event_count: restore_events,
            }
        );
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
