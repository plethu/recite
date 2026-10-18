use super::*;
use crate::buffers::{Bookmarks, Buffers};
use freya_testing::prelude::*;
use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{Document, Workbench};

#[test]
fn history_publication_updates_both_widgets_and_source_language_even_when_text_matches() {
    fn app() -> impl IntoElement {
        let buffers = Buffers {
            bookmarks: Bookmarks::new(),
            rules: use_state(|| None),
            model: use_state(|| Workbench::new("")),
            editor: use_state(|| crate::editing::editor_data("", false, false)),
            prose: use_state(String::new),
        };
        use_hook(move || {
            exercise_history(buffers)
                .unwrap_or_else(|error| panic!("history publication fixture: {error}"));
        });
        label().text("History published")
    }
    let _ = TestingRunner::new(app, (900., 650.).into(), |_| {}, 1.);
}

fn exercise_history(buffers: Buffers) -> Result<(), Box<dyn std::error::Error>> {
    for (before, after, view) in [
        ("", ":: one default\n-> END\n", View::Block("one".into())),
        (
            ":: one default\n> first@11111111111111111111\n  Before.\n-> END\n",
            ":: one default\n> first@11111111111111111111\n  After.\n-> END\n",
            View::Passage("11111111111111111111".into()),
        ),
    ] {
        let mut model = Workbench::from_document(Document::new(before)?, Some(View::Source))?;
        model.set_draft(after.into());
        model.apply()?;
        model.select(view)?;
        buffers.install(model, false);
        assert!(buffers.editor.peek().language.is_none());
        let request = buffers
            .model
            .peek()
            .as_ref()
            .map_err(|e| e.to_string())?
            .edit_request(EditOperation::Undo)?
            .ok_or("history entry")?;
        let mut prepared = request.prepare(&CancellationToken::new())?;
        publish(buffers, false, &mut prepared)?;
        let state = buffers.model.peek();
        let model = state.as_ref().map_err(|e| e.to_string())?;
        let source = model.view() == &View::Source;
        assert_eq!(model.document().source(), before);
        assert!(!model.has_draft());
        assert!(
            model
                .draft_snapshot()
                .matches_rope(&buffers.editor.peek().rope)
        );
        assert_eq!(
            buffers.prose.peek().as_str(),
            crate::buffers::prose_draft(model)
        );
        assert_eq!(buffers.editor.peek().language.is_some(), source);
        if source {
            assert_eq!(model.draft(), "");
        } else {
            assert_eq!(model.draft(), "Before.");
        }
    }
    Ok(())
}
