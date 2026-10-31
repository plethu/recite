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
        assert_eq!(
            first_summary.event_count,
            project.full_traversal_count()?,
            "{fixture} count-only traversal diverged"
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
                    output_count: 76,
                    event_hash: "cda4113e6dbbafe39d75e7c3a58b94516e7ed35ff94314fc55641ff66306c696"
                        .to_owned(),
                    state_hash: "fc8a639b66a90dd2aaf48221ec8b6cdc6af0e90c9916ebf1195e22e6fcd7f553"
                        .to_owned(),
                },
                PreviewSnapshotShape {
                    encoded_bytes: 3869,
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
                    output_count: 28,
                    event_hash: "a4ea4d02c0a12e356feb94612e62ad08937981c29d5ff21175c51120c7836bf3"
                        .to_owned(),
                    state_hash: "174ccd43b392bfab797b46990b1eac9f460e538f8c17778561bcef4dfa4a2996"
                        .to_owned(),
                },
                PreviewSnapshotShape {
                    encoded_bytes: 3623,
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
