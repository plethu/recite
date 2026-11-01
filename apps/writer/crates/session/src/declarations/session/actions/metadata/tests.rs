use super::*;
use crate::declarations::session::{
    parse,
    tests::{SOURCE, session},
};
use std::fs;

#[test]
fn output_reload_preserves_source_typing_and_remains_busy_through_retirement() {
    let (_directory, mut session) = session().unwrap();
    let old = session.schema_output.clone();
    let updated = parse(Path::new("schema.toml"), &SOURCE.replace("Mara", "Updated"))
        .unwrap()
        .export_json();
    fs::write(&session.output, updated).unwrap();
    let mut receipt = session
        .prepare_declaration(DeclarationAction::ReloadGenerated)
        .unwrap()
        .execute();
    assert!(!receipt.inspects_source());
    assert!(receipt.reloads_output());
    assert!(receipt.draft().is_none());
    session
        .source
        .as_mut()
        .unwrap()
        .set_draft("Newer unfinished TOML [".into())
        .unwrap();
    let finish = session.accept_declaration(&mut receipt).unwrap();
    assert!(session.busy());
    assert!(!Arc::ptr_eq(&old, &session.schema_output));
    assert_eq!(
        session.last_valid_schema().speakers["mara"]
            .display_name
            .as_deref(),
        Some("Updated")
    );
    session.cancel();
    session
        .accept_declaration_finish(&mut finish.execute())
        .unwrap();
    assert!(!session.busy());
    assert_eq!(
        session.source.as_ref().unwrap().draft,
        "Newer unfinished TOML ["
    );
    assert!(session.dirty());
}

#[test]
fn invalid_registration_is_adopted_as_state_without_rebasing_the_source() {
    let (directory, mut session) = session().unwrap();
    fs::write(
        directory
            .path()
            .join(recite_config::PRODUCER_REGISTRATION_FILE),
        "invalid TOML [",
    )
    .unwrap();
    let mut receipt = session
        .prepare_declaration(DeclarationAction::ReloadRegistration)
        .unwrap()
        .execute();
    assert!(receipt.registration().unwrap().is_err());
    session
        .source
        .as_mut()
        .unwrap()
        .set_draft("New source draft [".into())
        .unwrap();
    let finish = session.accept_declaration(&mut receipt).unwrap();
    assert!(session.registration.is_err());
    assert!(session.busy());
    session
        .accept_declaration_finish(&mut finish.execute())
        .unwrap();
    assert!(session.accept_declaration(&mut receipt).is_err());
    assert_eq!(session.source.as_ref().unwrap().draft, "New source draft [");
    fs::remove_file(
        directory
            .path()
            .join(recite_config::PRODUCER_REGISTRATION_FILE),
    )
    .unwrap();
    session.reload_registration();
    assert!(matches!(session.registration, Ok(None)));
}

#[test]
fn strict_output_reload_rejects_damage_foreign_ownership_and_cancelled_receipts() {
    let (_directory, mut session) = session().unwrap();
    let old = session.schema_output.clone();
    let foreign = parse(
        Path::new("schema.toml"),
        &SOURCE.replace("dialogue", "foreign"),
    )
    .unwrap()
    .export_json();
    for text in ["unfinished JSON {".to_owned(), foreign] {
        fs::write(&session.output, text).unwrap();
        assert!(session.reload_generated().is_err());
        assert!(!session.busy());
        assert!(Arc::ptr_eq(&old, &session.schema_output));
    }
    fs::write(&session.output, old.baseline()).unwrap();
    let mut receipt = session
        .prepare_declaration(DeclarationAction::ReloadGenerated)
        .unwrap()
        .execute();
    session.cancel();
    assert!(matches!(
        session.accept_declaration(&mut receipt),
        Err(FileError::DeclarationActionCancelled)
    ));
    assert!(Arc::ptr_eq(&old, &session.schema_output));
}
