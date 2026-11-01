use super::*;
use crate::declarations::session::{
    FileError, parse,
    tests::{SOURCE, session},
};
use std::path::Path;

fn generated() -> Generated {
    let parsed = parse(Path::new("schema.toml"), &SOURCE.replace("Mara", "Updated")).unwrap();
    Generated {
        text: parsed.export_json(),
        schema: parsed.schema().clone().into(),
    }
}

#[test]
fn publication_receipt_has_one_owner_and_one_acceptance() {
    let (_first_dir, mut first) = session().unwrap();
    let (_second_dir, mut second) = session().unwrap();
    let work = first.begin_publication(generated());
    let second_work = second.begin_publication(generated());
    let mut receipt = work.execute();
    assert!(second.accept_generation(&mut receipt).is_err());
    assert!(first.busy());
    assert!(second.busy());
    first.accept_generation(&mut receipt).unwrap();
    assert!(!first.busy());
    assert!(first.accept_generation(&mut receipt).is_err());
    second
        .accept_generation(&mut second_work.not_started("Worker stopped".into()))
        .unwrap_err();
    assert!(!second.busy());
}

#[test]
fn cancellation_before_write_retains_output_until_terminal_receipt() {
    let (_dir, mut session) = session().unwrap();
    let previous = std::fs::read_to_string(&session.output).unwrap();
    let work = session.begin_publication(generated());
    session.cancel();
    assert!(session.busy());
    let mut receipt = work.execute();
    assert!(session.accept_generation(&mut receipt).is_err());
    assert!(!session.busy());
    assert_eq!(std::fs::read_to_string(&session.output).unwrap(), previous);
    assert_eq!(session.schema_output.baseline(), previous);
}

#[test]
fn cancellation_after_write_reconciles_the_successful_prefix() {
    let (_dir, mut session) = session().unwrap();
    let expected = generated().text;
    let mut receipt = session.begin_publication(generated()).execute();
    session.cancel();
    assert!(session.busy());
    session.accept_generation(&mut receipt).unwrap();
    assert_eq!(session.schema_output.baseline(), expected);
    assert_eq!(std::fs::read_to_string(&session.output).unwrap(), expected);
    assert!(session.last_valid_schema().speakers.contains_key("mara"));
    assert!(!session.busy());
}

#[test]
fn output_conflict_and_dispatch_failure_both_release_publication() {
    let (_dir, mut session) = session().unwrap();
    let baseline = session.schema_output.baseline().to_owned();
    let work = session.begin_publication(generated());
    std::fs::write(&session.output, "external edit").unwrap();
    assert!(session.accept_generation(&mut work.execute()).is_err());
    assert_eq!(session.schema_output.baseline(), baseline);
    assert_eq!(
        std::fs::read_to_string(&session.output).unwrap(),
        "external edit"
    );
    assert!(!session.busy());
    let work = session.begin_publication(generated());
    assert!(
        session
            .accept_generation(&mut work.not_started("Worker unavailable".into()))
            .is_err()
    );
    assert!(!session.busy());
}

#[test]
fn publication_excludes_source_mutations_and_another_generation() {
    let (dir, mut session) = session().unwrap();
    let work = session.begin_publication(generated());
    assert!(matches!(session.discard(), Err(FileError::UnsavedDocument)));
    assert!(matches!(
        session.reload_source(),
        Err(FileError::UnsavedDocument)
    ));
    assert!(matches!(
        session.reload_generated(),
        Err(FileError::UnsavedDocument)
    ));
    assert!(matches!(
        session.bind(&dir.path().join("schema.toml")),
        Err(FileError::UnsavedDocument)
    ));
    assert!(session.prepare_save().is_err());
    assert!(session.start_generation().is_err());
    assert!(
        session
            .accept_generation(&mut work.not_started("Retired".into()))
            .is_err()
    );
}
