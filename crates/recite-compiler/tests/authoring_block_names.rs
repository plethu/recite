use recite_compiler::authoring::{
    AuthoringKernel, AuthoringRequest, CancellationToken, Interrupted, QueryClass, QueryResult,
    QueryUnavailableReason, SavedDocument, WorkControl,
};
use recite_core::DocumentKey;

#[test]
fn borrowed_block_names_preserve_document_and_source_order()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [
            SavedDocument::new(DocumentKey::new("z.recite")?, ":: last\n-> END\n"),
            SavedDocument::new(
                DocumentKey::new("a.recite")?,
                ":: second\n-> END\n:: first\n-> END\n:: second\n-> END\n",
            ),
        ],
        [],
    ))?;
    let control = CancellationToken::new();
    let query = kernel.snapshot().query(&control);
    let QueryResult::Ready(names) = query.project_block_names() else {
        panic!("complete declarations");
    };
    assert_eq!(
        names.iter().map(|name| name.as_str()).collect::<Vec<_>>(),
        ["second", "first", "second", "last"],
    );
    assert!(std::ptr::eq(
        names[0],
        kernel.snapshot().documents()[0].summary().blocks()[0].id(),
    ));
    Ok(())
}

#[test]
fn incomplete_block_names_preserve_recovered_values_and_authority()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("source.recite")?,
            ":: good\n-> END\n::\n",
        )],
        [],
    ))?;
    let control = CancellationToken::new();
    let query = kernel.snapshot().query(&control);
    let QueryResult::Partial { value, unavailable } = query.project_block_names() else {
        panic!("recoverable incomplete declarations");
    };
    assert_eq!(
        value.iter().map(|name| name.as_str()).collect::<Vec<_>>(),
        ["good"]
    );
    assert_eq!(
        unavailable,
        [QueryUnavailableReason::Incomplete(
            QueryClass::BlockDefinitions
        )],
    );
    assert_eq!(
        unavailable,
        query.project_block_symbols().unavailable_reasons()
    );
    Ok(())
}

#[test]
fn borrowed_block_names_stop_at_each_checkpoint_without_changing_the_snapshot()
-> Result<(), Box<dyn std::error::Error>> {
    let mut kernel = AuthoringKernel::new();
    kernel.apply(AuthoringRequest::new(
        kernel.snapshot().generation(),
        [SavedDocument::new(
            DocumentKey::new("source.recite")?,
            ":: first\n-> END\n:: second\n-> END\n:: third\n-> END\n",
        )],
        [],
    ))?;
    for cutoff in 0..4 {
        let control = StopAfter(std::cell::Cell::new(cutoff));
        assert_eq!(
            kernel.snapshot().query(&control).project_block_names(),
            QueryResult::Unavailable(vec![QueryUnavailableReason::Interrupted]),
        );
    }
    let control = CancellationToken::new();
    let query = kernel.snapshot().query(&control);
    let QueryResult::Ready(names) = query.project_block_names() else {
        panic!("fresh query after cancellation");
    };
    assert_eq!(
        names.iter().map(|name| name.as_str()).collect::<Vec<_>>(),
        ["first", "second", "third"]
    );
    Ok(())
}

struct StopAfter(std::cell::Cell<usize>);

impl WorkControl for StopAfter {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        match self.0.get().checked_sub(1) {
            Some(remaining) => {
                self.0.set(remaining);
                Ok(())
            }
            None => Err(Interrupted),
        }
    }
}
