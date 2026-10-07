use recite_compiler::authoring::{AuthoringError, Interrupted, SavedDocument, WorkControl};
use recite_core::DocumentKey;
use recite_writer_model::{
    Document, EditError, ProjectContext, SearchIndex, Workbench, WorkbenchError,
};
use std::cell::Cell;

struct Control {
    calls: Cell<usize>,
    stop_at: usize,
}

impl Control {
    fn new(stop_at: usize) -> Self {
        Self {
            calls: Cell::new(0),
            stop_at,
        }
    }
}

impl WorkControl for Control {
    fn checkpoint(&self) -> Result<(), Interrupted> {
        let call = self.calls.get();
        self.calls.set(call + 1);
        if call >= self.stop_at {
            Err(Interrupted)
        } else {
            Ok(())
        }
    }
}

fn context() -> Result<ProjectContext, Box<dyn std::error::Error>> {
    Ok(ProjectContext {
        documents: vec![SavedDocument::new(
            DocumentKey::new("other.recite")?,
            ":: destination\n-> END\n",
        )],
        schema: None,
    })
}

#[test]
fn initial_analysis_can_stop_at_each_checkpoint() -> Result<(), Box<dyn std::error::Error>> {
    let key = DocumentKey::new("scene.recite")?;
    let source = ":: start default\n-> other.recite::destination\n";
    let complete = Control::new(usize::MAX);
    let document = Document::in_project_with_control(key.clone(), source, context()?, &complete)?;
    assert!(document.diagnostics().is_empty());
    assert!(complete.calls.get() > 1, "analysis must check beyond entry");
    for stop_at in 0..complete.calls.get() {
        assert!(
            matches!(
                Document::in_project_with_control(
                    key.clone(),
                    source,
                    context()?,
                    &Control::new(stop_at)
                ),
                Err(EditError::Authoring(AuthoringError::Interrupted(_)))
            ),
            "published a document after checkpoint {stop_at} was interrupted"
        );
    }
    Ok(())
}

#[test]
fn interrupted_refresh_preserves_draft_preview_and_analysis()
-> Result<(), Box<dyn std::error::Error>> {
    let complete = Control::new(usize::MAX);
    let mut reference = Workbench::new(recite_writer_model::FIXTURE)?;
    reference.refresh_project_with_control(context()?, &complete)?;
    assert!(complete.calls.get() > 1);
    for stop_at in 0..complete.calls.get() {
        let mut workbench = Workbench::new(recite_writer_model::FIXTURE)?;
        workbench.start_preview()?;
        workbench.set_draft("Unapplied prose.".into());
        let revision = workbench.document().revision();
        let diagnostics = workbench.document().diagnostics();
        let sections = workbench.document().project_sections();
        let page = workbench.preview_page().ok_or("page")?.text.clone();
        let stale = workbench.preview_stale();
        assert!(matches!(
            workbench.refresh_project_with_control(context()?, &Control::new(stop_at)),
            Err(WorkbenchError::Edit(EditError::Authoring(
                AuthoringError::Interrupted(_)
            )))
        ));
        assert_eq!(workbench.document().revision(), revision);
        assert_eq!(workbench.document().diagnostics(), diagnostics);
        assert_eq!(workbench.document().project_sections(), sections);
        assert_eq!(workbench.document().source(), recite_writer_model::FIXTURE);
        assert_eq!(workbench.draft(), "Unapplied prose.");
        assert_eq!(workbench.preview_page().ok_or("page")?.text, page);
        assert_eq!(workbench.preview_stale(), stale);
    }
    Ok(())
}

#[test]
fn search_cancellation_never_returns_a_partial_index() -> Result<(), Box<dyn std::error::Error>> {
    let documents = vec![
        SavedDocument::new(DocumentKey::new("a.recite")?, recite_writer_model::FIXTURE),
        SavedDocument::new(DocumentKey::new("b.recite")?, recite_writer_model::FIXTURE),
    ];
    let complete = Control::new(usize::MAX);
    let controlled = SearchIndex::build_with_control(&documents, &complete)?;
    assert_eq!(
        controlled.search("would", 10),
        SearchIndex::build(&documents).search("would", 10)
    );
    for stop_at in 0..complete.calls.get() {
        assert!(matches!(
            SearchIndex::build_with_control(&documents, &Control::new(stop_at)),
            Err(Interrupted)
        ));
    }
    assert!(matches!(
        SearchIndex::build_with_control(&[], &Control::new(0)),
        Err(Interrupted)
    ));
    Ok(())
}

#[test]
fn large_document_search_stops_between_roots() -> Result<(), Box<dyn std::error::Error>> {
    let source = (0..128)
        .map(|index| {
            format!(":: block_{index}\n> line@{index:020}\n  Indexed prose {index}.\n-> END\n")
        })
        .collect::<String>();
    let documents = [SavedDocument::new(
        DocumentKey::new("large.recite")?,
        source,
    )];
    assert_eq!(SearchIndex::build(&documents).search("Indexed", 1).0, 128);
    for stop_at in [7, 32, 250] {
        let control = Control::new(stop_at);
        assert!(matches!(
            SearchIndex::build_with_control(&documents, &control),
            Err(Interrupted)
        ));
        assert_eq!(control.calls.get(), stop_at + 1);
    }
    Ok(())
}

#[test]
fn initial_selection_preserves_passage_block_and_source_fallbacks()
-> Result<(), Box<dyn std::error::Error>> {
    for (source, expected) in [
        (
            ":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n",
            recite_writer_model::View::Passage("11111111111111111111".into()),
        ),
        (
            ":: start default\n-> END\n",
            recite_writer_model::View::Block("start".into()),
        ),
        ("", recite_writer_model::View::Source),
        ("malformed syntax", recite_writer_model::View::Source),
        (
            ":: start default\n> line\n  Unfrozen.\n-> END\n",
            recite_writer_model::View::Source,
        ),
    ] {
        let workbench = Workbench::new(source)?;
        assert_eq!(workbench.view(), &expected, "{source}");
    }
    Ok(())
}
