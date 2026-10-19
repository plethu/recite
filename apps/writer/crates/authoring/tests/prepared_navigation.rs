use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{Document, PreparedNavigation, View, Workbench};
use std::sync::Arc;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = ":: start default\n> line@11111111111111111111\n  Before.\n-> END\n\n:: later\n> other@22222222222222222222\n  Later.\n-> END\n";
fn model() -> Result<Workbench> {
    Ok(Workbench::from_document(
        Document::new(SOURCE)?,
        Some(View::Source),
    )?)
}
fn prepare(model: &Workbench) -> Result<PreparedNavigation> {
    let request = model.navigation_request();
    Ok(
        std::thread::spawn(move || request.prepare(None, &CancellationToken::new()))
            .join()
            .map_err(|_| "navigation preparation panicked")??,
    )
}
#[test]
fn navigation_preserves_history_preview_and_unapplied_source() -> Result {
    let mut model = model()?;
    model.select(View::Passage("11111111111111111111".into()))?;
    model.set_draft("Accepted.".into());
    model.apply()?;
    model.start_preview()?;
    let page = model.preview_page().cloned();
    let history = model.document().history_bytes();
    let mut prepared = prepare(&model)?;
    prepared.model_mut().inspect_block("later")?;
    model.accept_navigation(&mut prepared)?;
    assert_eq!(model.view(), &View::Block("later".into()));
    assert_eq!(
        model.preview_page().map(|p| (&p.text, p.ended)),
        page.as_ref().map(|p| (&p.text, p.ended))
    );
    assert_eq!(model.document().history_bytes(), history);
    assert!(model.accept_navigation(&mut prepared).is_err());
    model.undo()?;
    assert_eq!(model.document().source(), SOURCE);
    model.select(View::Source)?;
    model.set_draft("Invalid recoverable Source draft".into());
    let source = model.document().source_snapshot();
    let mut prepared = prepare(&model)?;
    model.accept_navigation(&mut prepared)?;
    assert!(Arc::ptr_eq(&source, &model.document().source_snapshot()));
    assert_eq!(model.draft(), "Invalid recoverable Source draft");
    assert!(model.has_draft());
    Ok(())
}
#[test]
fn stale_or_edited_candidate_cannot_publish_or_advance_live_history() -> Result {
    for change in ["draft", "view", "context", "candidate"] {
        let mut model = model()?;
        let mut prepared = prepare(&model)?;
        match change {
            "draft" => model.set_draft("Newer typing".into()),
            "view" => model.inspect_block("later")?,
            "context" => model.refresh_project(Default::default())?,
            _ => {
                prepared
                    .model_mut()
                    .set_draft(format!("{SOURCE}\n# New candidate source\n"));
                prepared.model_mut().apply()?;
            }
        }
        let source = model.document().source_snapshot();
        let history = model.document().history_bytes();
        assert!(!prepared.is_current(&model));
        assert!(model.accept_navigation(&mut prepared).is_err());
        assert!(Arc::ptr_eq(&source, &model.document().source_snapshot()));
        assert_eq!(model.document().history_bytes(), history);
    }
    Ok(())
}
#[test]
fn cancelled_preparation_does_not_touch_the_live_document() -> Result {
    let model = model()?;
    let token = CancellationToken::new();
    token.interrupt();
    assert!(model.navigation_request().prepare(None, &token).is_err());
    assert_eq!(model.document().source(), SOURCE);
    assert!(!model.document().can_undo());
    Ok(())
}

#[test]
fn rules_belong_to_the_exact_accepted_source_even_when_key_and_revision_repeat() -> Result {
    let source = ":: start default\n? reply@11111111111111111111\n  Continue.\n  -> END\n";
    let first = Document::new(source)?;
    let rules = first.reply_rules("11111111111111111111")?;
    assert!(rules.belongs_to(&first));
    let reopened = Document::new(source)?;
    assert_eq!(first.key(), reopened.key());
    assert_eq!(first.revision(), reopened.revision());
    assert!(!rules.belongs_to(&reopened));
    assert!(rules.validate_draft(&reopened).is_err());
    Ok(())
}

#[test]
fn sibling_edits_rebind_staged_rules_but_schema_changes_leave_them_explicitly_stale() -> Result {
    use recite_compiler::authoring::SavedDocument;
    use recite_writer_model::ProjectContext;
    let source = ":: start default\n? reply@11111111111111111111 requires=(ready())\n  Continue.\n  -> END\n";
    for changed_schema in [false, true] {
        let document = Document::new(source)?;
        let mut rules = document.reply_rules("11111111111111111111")?;
        rules.condition = None;
        let staged = rules.source()?;
        let mut model = Workbench::from_document(document, Some(View::Source))?;
        model.set_draft(staged.clone());
        let schema = if changed_schema {
            recite_core::schema::load_schema_manifest_str(
                "schema.json",
                include_str!("../../../../../fixtures/schema/valid/generated_manifest.json"),
            )
            .schema
        } else {
            None
        };
        let context = ProjectContext {
            documents: vec![SavedDocument::new(
                recite_core::DocumentKey::new("other.recite")?,
                ":: other\n-> END\n",
            )],
            schema,
        };
        let mut prepared = model
            .navigation_request()
            .prepare(Some(context), &CancellationToken::new())?;
        assert!(!prepared.refresh_rules(&mut rules, model.document()));
        model.accept_navigation(&mut prepared)?;
        assert_eq!(
            prepared.refresh_rules(&mut rules, model.document()),
            !changed_schema
        );
        assert_eq!(model.document().source(), source);
        assert_eq!(model.draft(), staged);
        assert!(model.has_draft());
        assert!(rules.changed());
        if changed_schema {
            assert!(rules.validate_draft(model.document()).is_err());
        } else {
            assert_eq!(rules.validate_draft(model.document())?, staged);
            model.apply_reply_rules(&rules)?;
            assert_eq!(model.document().source(), staged);
            model.undo()?;
            assert_eq!(model.document().source(), source);
        }
    }
    Ok(())
}
