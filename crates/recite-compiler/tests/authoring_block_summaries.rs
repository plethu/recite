//! Navigation facts come from accepted syntax, including incremental edits and recovery.
use recite_compiler::authoring::{AuthoringKernel, AuthoringRequest, SavedDocument};
use recite_core::DocumentKey;

fn update(kernel: &mut AuthoringKernel, source: &str) -> Result<(), Box<dyn std::error::Error>> {
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("scene.recite")?,
            source,
        )],
        [],
    ))?;
    Ok(())
}

#[test]
fn badges_describe_authored_destinations_without_inferring_reachability()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    update(
        &mut kernel,
        concat!(
            ":: first default\n-> next\n# -> END\n",
            ":: next\n> line@11111111111111111111\n  Say END.\n  ? choice@22222222222222222222\n    Finish.\n    -> END\n",
            ":: conditional\n:if ready()\n  -> END\n",
            ":: matched\n:match mood()\n  :case _\n    -> END\n",
            ":: unreachable\n-> first\n-> END\n",
        ),
    )?;
    let blocks = kernel.snapshot().documents()[0].summary().blocks();
    assert_eq!(
        blocks
            .iter()
            .map(|b| (b.id().as_str(), b.is_default(), b.has_end_destination()))
            .collect::<Vec<_>>(),
        [
            ("first", true, false),
            ("next", false, true),
            ("conditional", false, true),
            ("matched", false, true),
            ("unreachable", false, true),
        ]
    );
    Ok(())
}

#[test]
fn edits_refresh_badges_and_spans_without_mutating_retained_summaries()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    let initial = ":: first default\n-> END\n:: next\n-> first\n";
    update(&mut kernel, initial)?;
    let retained_snapshot = kernel.snapshot().documents()[0].summary_snapshot();
    assert!(std::sync::Arc::ptr_eq(
        &retained_snapshot,
        &kernel.snapshot().documents()[0].summary_snapshot()
    ));
    let retained = retained_snapshot.blocks();
    for source in [
        ":: first\n-> next\n:: next default\n-> END\n",
        "# relocate\n:: first\n-> next\n:: next default\n-> END\n",
        initial,
        ":: first\n-> END\n::\n",
    ] {
        update(&mut kernel, source)?;
        let mut fresh = AuthoringKernel::new();
        update(&mut fresh, source)?;
        assert_eq!(
            kernel.snapshot().documents()[0].summary().blocks(),
            fresh.snapshot().documents()[0].summary().blocks()
        );
        assert!(retained[0].is_default());
        assert!(retained[0].has_end_destination());
        assert!(!retained[1].is_default());
        assert!(!retained[1].has_end_destination());
        assert_eq!(
            retained[0].span().start,
            recite_core::SourcePosition::new(1, 1)?
        );
    }
    Ok(())
}
