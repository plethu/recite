use super::*;
use freya::code_editor::Rope;
use recite_compiler::authoring::CancellationToken;
use recite_core::SourcePosition;
use recite_writer_model::{Document, ProjectContext};

const SOURCE: &str = ":: start default\n-> dest\n:: destination\n-> END\n";
fn request(document: &Document, source: &str) -> Arc<Request> {
    Arc::new(Request {
        analysis: document.completion_snapshot(),
        draft: Rope::from_str(source),
        position: SourcePosition::new(2, 8).unwrap(),
    })
}
fn completed(request: &Arc<Request>) -> Arc<Completed> {
    let source = String::from(&request.draft);
    Arc::new(Completed {
        request: Arc::clone(request),
        items: request
            .analysis
            .complete(&source, request.position, &CancellationToken::new())
            .unwrap(),
    })
}
#[test]
fn results_belong_to_the_current_request_and_cannot_resurrect_after_close() {
    let document = Document::new(SOURCE).unwrap();
    let first = request(&document, SOURCE);
    let latest = request(&document, &format!("{SOURCE}# changed\n"));
    let mut cache = SourceCompletionCache::new();
    cache.current = Some(Current {
        request: Arc::clone(&latest),
        result: ResultState::Pending,
    });
    assert!(cache.pending());
    assert!(!cache.accept(Output::Completed(completed(&first))).0);
    assert!(cache.pending());
    let result = completed(&latest);
    assert!(cache.accept(Output::Completed(Arc::clone(&result))).0);
    assert!(!cache.pending());
    assert!(cache.owns(&result));
    assert!(!cache.owns(&completed(&latest)));
    cache.clear();
    assert!(!cache.owns(&result));
    assert!(!cache.accept(Output::Completed(result)).0);
    // Identical bytes after reopening still have a distinct publication authority.
    let reopened = request(&document, &String::from(&latest.draft));
    cache.current = Some(Current {
        request: reopened,
        result: ResultState::Pending,
    });
    assert!(!cache.accept(Output::Completed(completed(&latest))).0);
    assert!(cache.pending());
}
#[test]
fn same_byte_context_refresh_cursor_and_raw_rope_changes_invalidate_requests() {
    let mut document = Document::new(SOURCE).unwrap();
    let first = request(&document, SOURCE);
    assert!(first.matches(&request(&document, SOURCE)));
    document.refresh_project(ProjectContext::default()).unwrap();
    assert!(!first.matches(&request(&document, SOURCE)));
    let mut cursor = Request {
        analysis: first.analysis.clone(),
        draft: first.draft.clone(),
        position: SourcePosition::new(2, 7).unwrap(),
    };
    assert!(!first.matches(&cursor));
    cursor.position = first.position;
    cursor.draft = Rope::from_str(&SOURCE.replace("dest\n", "diff\n"));
    assert!(!first.matches(&cursor));
}
#[test]
fn failed_work_stops_pending_feedback_and_old_receivers_cannot_publish() {
    let document = Document::new(SOURCE).unwrap();
    let mut cache = SourceCompletionCache::new();
    let receiver = cache.results();
    assert!(cache.matches_receiver(&receiver));
    assert!(!SourceCompletionCache::new().matches_receiver(&receiver));
    let current = request(&document, SOURCE);
    cache.current = Some(Current {
        request: Arc::clone(&current),
        result: ResultState::Pending,
    });
    let stale = request(&document, SOURCE);
    assert!(
        !cache
            .accept(Output::Failed {
                request: Some(stale),
                message: "obsolete".into()
            })
            .0
    );
    let (changed, error) = cache.accept(Output::Failed {
        request: Some(current),
        message: "failed query".into(),
    });
    assert!(changed);
    assert_eq!(error.as_deref(), Some("failed query"));
    assert!(!cache.pending());
    cache.current.as_mut().unwrap().result = ResultState::Pending;
    let (changed, error) = cache.accept(Output::Failed {
        request: None,
        message: "worker failed".into(),
    });
    assert!(changed);
    assert_eq!(error.as_deref(), Some("worker failed"));
    assert!(!cache.pending());
}
