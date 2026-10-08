use crate::authoring::CancellationToken;
use crate::authoring::{
    AuthoringEditError, AuthoringKernel, AuthoringRequest, QueryUnavailableReason, SavedDocument,
    SymbolQueryOptions, WorkControl,
};
use recite_core::{DocumentKey, SourcePosition};

#[test]
fn project_queries_and_edit_plans_stop_without_poisoning_the_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    let a = DocumentKey::new("a.recite")?;
    let b = DocumentKey::new("b.recite")?;
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [
            SavedDocument::new(a.clone(), ":: start\n>\n  Hello.\n-> b.recite::target\n"),
            SavedDocument::new(b.clone(), ":: target\n>\n  World.\n-> END\n"),
        ],
        [],
    ))?;
    let snapshot = kernel.snapshot();
    let expected_symbols = snapshot.project_symbols(SymbolQueryOptions::default());
    let expected_rename = snapshot.plan_rename_block(&b, SourcePosition::new(1, 4)?, "renamed")?;
    let expected_ids = snapshot.plan_insert_missing_ids()?;
    let mut interrupted_rename = 0;
    let mut interrupted_ids = 0;
    for cutoff in 0..64 {
        let control = CancellationToken::interrupt_after(cutoff);
        let result = snapshot
            .query(&control)
            .project_symbols(SymbolQueryOptions::default());
        if control.checkpoint().is_ok() {
            assert_eq!(result, expected_symbols);
        } else if cutoff == 0 {
            assert!(
                result
                    .unavailable_reasons()
                    .contains(&QueryUnavailableReason::Interrupted)
            );
        }
        let control = CancellationToken::interrupt_after(cutoff);
        match snapshot
            .query(&control)
            .plan_rename_block(&b, SourcePosition::new(1, 4)?, "renamed")
        {
            Err(AuthoringEditError::Interrupted(_)) => interrupted_rename += 1,
            result => assert_eq!(result?, expected_rename),
        }
        let control = CancellationToken::interrupt_after(cutoff);
        match snapshot.query(&control).plan_insert_missing_ids() {
            Err(AuthoringEditError::Interrupted(_)) => interrupted_ids += 1,
            result => assert_eq!(result?, expected_ids),
        }
    }
    assert!(interrupted_rename > 4 && interrupted_ids > 4);
    assert_eq!(
        snapshot.plan_rename_block(&b, SourcePosition::new(1, 4)?, "renamed")?,
        expected_rename
    );
    assert_eq!(snapshot.plan_insert_missing_ids()?, expected_ids);
    Ok(())
}

#[test]
fn block_projection_preserves_recoverable_symbols_and_source_order()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::authoring::{QueryResult, SymbolKind};
    for source in [
        ":: start\n-> END\n",
        ":: good\n-> END\n::\n",
        ":: second\n:: first\n",
    ] {
        let mut kernel = AuthoringKernel::new();
        kernel.apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            [SavedDocument::new(
                DocumentKey::new("source.recite")?,
                source,
            )],
            [],
        ))?;
        let values = |result| match result {
            QueryResult::Ready(values) | QueryResult::Partial { value: values, .. } => values,
            _ => panic!("recoverable symbols"),
        };
        let expected = values(
            kernel
                .snapshot()
                .project_symbols(SymbolQueryOptions::default()),
        )
        .into_iter()
        .filter(|symbol| symbol.kind() == SymbolKind::Block)
        .collect::<Vec<_>>();
        assert_eq!(values(kernel.snapshot().project_block_symbols()), expected);
    }
    Ok(())
}
