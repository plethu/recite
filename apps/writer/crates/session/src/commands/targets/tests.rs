use super::*;
use crate::commands::search::repeat_target;
use criterion::measurement::{Measurement, WallTime};
use recite_writer_model::{Document, ProjectContext, workload};
use std::cell::Cell;
use std::collections::BTreeSet;

#[test]
fn repeat_targets_are_unique_and_ranking_retains_source_order_within_each_rank()
-> Result<(), Box<dyn std::error::Error>> {
    let mut model = Workbench::new(":: start default\n-> END\n:: another\n-> END\n")?;
    let generated = project(&model);
    let unique = generated
        .iter()
        .map(|target| (&target.document, &target.beat))
        .collect::<BTreeSet<_>>();
    assert_eq!(generated.len(), unique.len());
    assert_eq!(
        matching(generated, "another")[0].beat.as_deref(),
        Some("another")
    );
    model.set_draft("unapplied lookup token".into());
    assert!(matching(project(&model), "token").is_empty());
    Ok(())
}

#[test]
fn repeated_lookup_ranks_only_accepted_navigation_facts_in_a_fixed_large_project()
-> Result<(), Box<dyn std::error::Error>> {
    let documents = workload::project(10_000, 500)?;
    let first = &documents[0];
    let document = Document::in_project(
        first.key().clone(),
        first.text(),
        ProjectContext {
            documents: documents.clone(),
            schema: None,
        },
    )?;
    let model = Workbench::from_document(document, None)?;
    let mut accepted = None;
    let visits = Cell::new(0);
    let started = WallTime.start();
    for _ in 0..64 {
        let matches = matching(
            project(&model)
                .into_iter()
                .inspect(|_| visits.set(visits.get() + 1)),
            "beat",
        );
        assert_eq!(matches.len(), 2_000);
        accepted = repeat_target(&matches, accepted.as_ref(), true).cloned();
    }
    let milliseconds = WallTime.end(started).as_secs_f64() * 1_000.;
    // One accepted-fact scan per command:20 scene targets+2,000 beat targets, independent of prose.
    assert_eq!(visits.get(), 64 * 2_020);
    assert_eq!(
        accepted.and_then(|target| target.beat),
        Some("beat_63".into())
    );
    println!("10,000 passages /2,000 beats,64 repeated lookups: {milliseconds:.2} ms");
    Ok(())
}
