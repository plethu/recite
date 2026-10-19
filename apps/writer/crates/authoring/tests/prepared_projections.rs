use recite_writer_model::{Document, PreparedProjection, ProjectionRequest};
use std::sync::Arc;

#[test]
fn worker_projection_matches_synchronous_output_and_shares_existing_passages()
-> Result<(), Box<dyn std::error::Error>> {
    fn send<T: Send>() {}
    send::<ProjectionRequest>();
    send::<PreparedProjection>();
    for warm_passages in [false, true] {
        let document = Document::new(recite_writer_model::FIXTURE)?;
        let expected = Document::new(recite_writer_model::FIXTURE)?;
        assert!(document.cached_passages().is_none());
        assert!(document.cached_script().is_none());
        let passages = warm_passages
            .then(|| document.passage_snapshot())
            .transpose()?;
        let request = document.projection_request();
        let prepared = std::thread::spawn(move || request.prepare().map_err(|e| e.to_string()))
            .join()
            .map_err(|_| "worker panicked")??;
        assert!(document.accept_projection(&prepared));
        assert_eq!(document.script_snapshot()?, expected.script_snapshot()?);
        assert_eq!(document.passage_snapshot()?, expected.passage_snapshot()?);
        let first = document.script_snapshot()?;
        assert!(document.accept_projection(&prepared));
        assert!(Arc::ptr_eq(&first, &document.script_snapshot()?));
        if let Some(passages) = passages {
            assert!(Arc::ptr_eq(&passages, &document.passage_snapshot()?));
        }
        for passage in document.passage_snapshot()?.iter() {
            assert_eq!(document.find_passage(&passage.id)?, Some(passage.clone()));
        }
    }
    Ok(())
}

#[test]
fn project_refresh_keeps_local_receipts_but_edits_and_reopen_reject_them()
-> Result<(), Box<dyn std::error::Error>> {
    let mut document = Document::new(recite_writer_model::FIXTURE)?;
    let request = document.projection_request();
    let prepared = request.prepare()?;
    document.refresh_project(Default::default())?;
    assert!(request.is_current(&document));
    assert!(request.same_source(&document.projection_request()));
    assert!(document.accept_projection(&prepared));
    let reopened = Document::new(recite_writer_model::FIXTURE)?;
    assert!(!reopened.accept_projection(&prepared));
    assert!(reopened.cached_script().is_none());
    document.replace_source(
        document.revision(),
        document
            .source()
            .replace("Would you tell me", "Could you tell me"),
    )?;
    assert!(!request.is_current(&document));
    assert!(!request.same_source(&document.projection_request()));
    assert!(!document.accept_projection(&prepared));
    assert!(document.cached_script().is_none());
    assert!(
        Document::new(":: broken\n? invalid\n")?
            .projection_request()
            .prepare()
            .is_err()
    );
    Ok(())
}

#[test]
fn queued_requests_do_not_retain_old_passage_trees() -> Result<(), Box<dyn std::error::Error>> {
    let document = Document::new(recite_writer_model::FIXTURE)?;
    let passages = document.passage_snapshot()?;
    let weak = Arc::downgrade(&passages);
    let request = document.projection_request();
    drop(passages);
    drop(document);
    assert!(weak.upgrade().is_none());
    // Preparation can reconstruct from its frozen source after cache retirement.
    assert!(request.prepare().is_ok());
    Ok(())
}
