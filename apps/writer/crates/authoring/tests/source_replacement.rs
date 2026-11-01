use recite_compiler::authoring::CancellationToken;
use recite_writer_model::{SourceValidation, View, Workbench};

const SOURCE: &str = ":: start default\n> @ffee6p8\n  Original.\n-> END\n";
const LINE: &str = "ffee6p8";

#[test]
fn full_source_adoption_retains_earlier_history_and_restores_the_resolved_field() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.select(View::Passage(LINE.into())).unwrap();
    model.set_draft("Earlier accepted edit.".into());
    model.apply().unwrap();
    let accepted = model.document().source().to_owned();
    model.set_draft("Unapplied field for the caller to archive.".into());
    let before = model.recovery();
    let disk = SOURCE.replace("Original.", "Inspected disk.");
    let mut prepared = model
        .source_replacement_request(SourceValidation::Freeform)
        .prepare(disk.clone().into(), &CancellationToken::new())
        .unwrap();
    assert_eq!(model.recovery(), before);
    assert_eq!(
        prepared.recovery().draft_snapshot().as_str(),
        "Inspected disk."
    );
    model.accept_source_replacement(&mut prepared).unwrap();
    assert_eq!(model.document().source(), disk);
    assert_eq!(model.view(), &View::Passage(LINE.into()));
    assert_eq!(model.draft(), "Inspected disk.");
    assert!(!model.has_draft());
    assert!(model.accept_source_replacement(&mut prepared).is_err());
    model.undo().unwrap();
    assert_eq!(model.document().source(), accepted);
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
    model.redo().unwrap();
    assert_eq!(model.document().source(), accepted);
    model.redo().unwrap();
    assert_eq!(model.document().source(), disk);
}

#[test]
fn inspected_malformed_source_remains_repairable_while_reviewed_import_stays_strict() {
    let mut model = Workbench::new(SOURCE).unwrap();
    let malformed = ":: start default\n? invalid";
    let before = model.recovery();
    assert!(
        model
            .source_replacement_request(SourceValidation::NoNewErrors)
            .prepare(malformed.to_owned().into(), &CancellationToken::new())
            .is_err()
    );
    assert_eq!(model.recovery(), before);
    let mut prepared = model
        .source_replacement_request(SourceValidation::Freeform)
        .prepare(malformed.to_owned().into(), &CancellationToken::new())
        .unwrap();
    model.accept_source_replacement(&mut prepared).unwrap();
    assert_eq!(model.document().source(), malformed);
    assert_eq!(model.view(), &View::Source);
    assert!(model.source_repair_request().is_ok());
    model.undo().unwrap();
    assert_eq!(model.document().source(), SOURCE);
}

#[test]
fn reviewed_merge_accepts_without_live_source_staging_and_newer_work_rejects() {
    let mut model = Workbench::new(SOURCE).unwrap();
    model.select(View::Passage(LINE.into())).unwrap();
    model.set_draft("Pending local field.".into());
    let merged = SOURCE.replace("Original.", "Reviewed merge.");
    let mut prepared = model
        .source_replacement_request(SourceValidation::NoNewErrors)
        .prepare(merged.clone().into(), &CancellationToken::new())
        .unwrap();
    model.set_draft("Newer local field.".into());
    let before = model.recovery();
    assert!(model.accept_source_replacement(&mut prepared).is_err());
    assert_eq!(model.recovery(), before);
    assert!(!model.document().can_undo());
    let mut prepared = model
        .source_replacement_request(SourceValidation::NoNewErrors)
        .prepare(merged.clone().into(), &CancellationToken::new())
        .unwrap();
    model.accept_source_replacement(&mut prepared).unwrap();
    assert_eq!(model.document().source(), merged);
    assert_eq!(model.view(), &View::Passage(LINE.into()));
    assert_eq!(model.source_validation(), SourceValidation::NoNewErrors);
}
