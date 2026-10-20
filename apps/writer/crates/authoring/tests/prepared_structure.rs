use recite_compiler::authoring::{CancellationToken, Interrupted, WorkControl};
use recite_writer_model::{PreparedStructuralEdit, RouteOwner, StructuralCommand, View, Workbench};
use std::sync::atomic::{AtomicUsize, Ordering};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const SOURCE: &str = "# Keep comments.\n:: start default\n> first@11111111111111111111\n  Before.\n-> later\n:: later\n> second@22222222222222222222\n  Later.\n-> END\n";
fn prepare(model: &Workbench, command: StructuralCommand) -> Result<PreparedStructuralEdit> {
    let request = model.structural_request(command)?;
    Ok(
        std::thread::spawn(move || request.prepare(&CancellationToken::new()))
            .join()
            .map_err(|_| "structural worker panicked")??,
    )
}
#[test]
fn structural_commands_preserve_earlier_history_and_each_create_one_live_undo_step() -> Result {
    for command in [
        StructuralCommand::AddBeat,
        StructuralCommand::AddLine,
        StructuralCommand::AddReply,
        StructuralCommand::Destination {
            owner: RouteOwner::Beat("start".into()),
            destination: "END".into(),
        },
        StructuralCommand::RenameBeat("renamed".into()),
    ] {
        let mut model = Workbench::new(SOURCE)?;
        model.set_draft("Earlier accepted prose.".into());
        model.apply()?;
        let previous = model.document().source().to_owned();
        let mut prepared = prepare(&model, command.clone())?;
        assert_eq!(model.document().source(), previous);
        model.accept_structural_edit(&mut prepared)?;
        assert!(!model.has_draft());
        assert!(model.document().source().contains("# Keep comments."));
        assert!(
            model
                .document()
                .source()
                .contains("first@11111111111111111111")
        );
        assert!(model.accept_structural_edit(&mut prepared).is_err());
        let after = model.document().source().to_owned();
        assert_ne!(after, previous, "{command:?}");
        model.undo()?;
        assert_eq!(model.document().source(), previous);
        model.undo()?;
        assert_eq!(model.document().source(), SOURCE);
        model.redo()?;
        model.redo()?;
        assert_eq!(model.document().source(), after);
    }
    Ok(())
}
#[test]
fn newer_typing_selection_and_context_reject_structural_publication_atomically() -> Result {
    for changed in ["draft", "selection", "context"] {
        let mut model = Workbench::new(SOURCE)?;
        let mut prepared = prepare(&model, StructuralCommand::AddLine)?;
        match changed {
            "draft" => model.set_draft("Newer typing stays open.".into()),
            "selection" => model.inspect_block("later")?,
            _ => model.refresh_project(Default::default())?,
        }
        let before = model.recovery();
        let source = model.document().source_snapshot();
        let history = model.document().history_bytes();
        assert!(model.accept_structural_edit(&mut prepared).is_err());
        assert_eq!(model.recovery(), before);
        assert!(std::sync::Arc::ptr_eq(
            &source,
            &model.document().source_snapshot()
        ));
        assert_eq!(model.document().history_bytes(), history);
    }
    Ok(())
}
#[test]
fn no_op_destination_preserves_revision_analysis_and_redo() -> Result {
    let mut model = Workbench::new(SOURCE)?;
    model.set_draft("Accepted then undone.".into());
    model.apply()?;
    model.undo()?;
    let revision = model.document().revision();
    let stamp = model.document().analysis_stamp();
    let mut prepared = prepare(
        &model,
        StructuralCommand::Destination {
            owner: RouteOwner::Beat("start".into()),
            destination: "later".into(),
        },
    )?;
    model.accept_structural_edit(&mut prepared)?;
    assert_eq!(model.document().revision(), revision);
    assert!(stamp.is_current(model.document()));
    assert!(model.document().can_redo());
    model.redo()?;
    assert!(model.document().source().contains("Accepted then undone."));
    Ok(())
}
struct StopAfterPlanning(AtomicUsize);
impl WorkControl for StopAfterPlanning {
    fn checkpoint(&self) -> std::result::Result<(), Interrupted> {
        if self.0.fetch_add(1, Ordering::Relaxed) == 0 {
            Ok(())
        } else {
            Err(Interrupted)
        }
    }
}
#[test]
fn interrupted_or_invalid_commands_leave_live_state_and_draft_unchanged() -> Result {
    let mut model = Workbench::new(SOURCE)?;
    let before = model.recovery();
    let request = model.structural_request(StructuralCommand::AddBeat)?;
    assert!(
        request
            .prepare(&StopAfterPlanning(AtomicUsize::new(0)))
            .is_err()
    );
    assert!(
        prepare(
            &model,
            StructuralCommand::Destination {
                owner: RouteOwner::Reply("11111111111111111111".into()),
                destination: "END".into()
            }
        )
        .is_err(),
        "a dialogue identity must never turn a destination edit into a speaker edit"
    );
    assert_eq!(model.document().source(), SOURCE);
    assert_eq!(model.recovery(), before);
    assert!(
        prepare(
            &model,
            StructuralCommand::Destination {
                owner: RouteOwner::Reply("missing".into()),
                destination: "END".into()
            }
        )
        .is_err()
    );
    model.set_draft("Pending text.".into());
    assert!(
        model
            .structural_request(StructuralCommand::AddLine)
            .is_err()
    );
    assert_eq!(model.draft(), "Pending text.");
    assert_eq!(model.view(), &View::Passage("11111111111111111111".into()));
    Ok(())
}
