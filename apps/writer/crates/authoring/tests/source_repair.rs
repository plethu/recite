use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{SourceValidation, View, Workbench};

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Original.\n-> END\n";
const INVALID: &str = ":: start default\n? invalid";

#[test]
fn explicit_repair_accepts_only_a_valid_proposal_and_retains_authoring_history() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.select(View::Source).unwrap();
    model.set_draft(INVALID.into());
    let before = model.recovery();
    let request = model.source_repair_request().unwrap();
    assert!(
        request
            .prepare(INVALID.to_owned().into(), &CancellationToken::new())
            .is_err()
    );
    assert_eq!(model.recovery(), before);

    let repaired = SOURCE.replace("Original.", "Repaired café 💬.");
    let request = model.source_repair_request().unwrap();
    let mut prepared = request
        .prepare(repaired.clone().into(), &CancellationToken::new())
        .unwrap();
    assert_eq!(model.recovery(), before, "preparation cannot publish");
    model.accept_source_repair(&mut prepared).unwrap();
    assert_eq!(model.document().source(), repaired);
    assert_eq!(model.draft(), repaired);
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
    assert!(!model.has_draft());
    assert!(model.accept_source_repair(&mut prepared).is_err());
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
}

#[test]
fn newer_typing_foreign_documents_and_cancellation_reject_without_publishing() {
    let mut model = Workbench::open("scene.recite", SOURCE).unwrap();
    model.select(View::Source).unwrap();
    model.set_draft(INVALID.into());
    let repaired = SOURCE.replace("Original.", "External repair.");
    let request = model.source_repair_request().unwrap();
    let mut prepared = request
        .prepare(repaired.clone().into(), &CancellationToken::new())
        .unwrap();
    let mut foreign = Workbench::open("other.recite", SOURCE).unwrap();
    foreign.select(View::Source).unwrap();
    foreign.set_draft(INVALID.into());
    let before = foreign.recovery();
    assert!(foreign.accept_source_repair(&mut prepared).is_err());
    assert_eq!(foreign.recovery(), before);
    model.set_draft("Newer recovered work.".into());
    let before = model.recovery();
    assert!(model.accept_source_repair(&mut prepared).is_err());
    assert_eq!(model.recovery(), before);
    let request = model.source_repair_request().unwrap();
    let control = CancellationToken::new();
    control.interrupt();
    assert!(request.prepare(repaired.into(), &control).is_err());
    assert_eq!(model.recovery(), before);
}
