use super::*;

#[test]
fn metadata_reload_keeps_source_preview_and_newer_manuscript_typing_independent() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.declaration_action(DeclarationAction::Bind(
        directory.path().join("schema.toml"),
    ));
    finish(&mut writer, &mut jobs);
    writer.change(|model| {
        model.set_draft("Unapplied manuscript.".into());
        Ok(())
    });
    fs::copy(
        directory.path().join("fixture.json"),
        directory.path().join("schema.json"),
    )
    .unwrap();
    writer.declaration_action(DeclarationAction::ReloadGenerated);
    finish(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().draft(), "Unapplied manuscript.");
    assert!(
        writer
            .status
            .contains("Schema reloaded; project analysis refreshed"),
        "{}",
        writer.status
    );
    assert!(
        writer
            .workspaces
            .declarations
            .source
            .status_message()
            .contains("Validate this draft")
    );
    writer.declaration_action(DeclarationAction::InspectSource);
    finish(&mut writer, &mut jobs);
    assert!(
        writer
            .workspaces
            .declarations
            .source
            .status_message()
            .contains("Save and generate")
    );
    assert!(
        writer
            .declaration_session()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .draft
            .contains("Mara")
    );
}

#[test]
fn reloaded_registration_updates_reviewed_command_or_error_without_touching_source() {
    let (directory, mut writer, mut jobs) = fixture();
    let registration = directory
        .path()
        .join(recite_config::PRODUCER_REGISTRATION_FILE);
    let original = fs::read_to_string(&registration).unwrap();
    fs::write(&registration, "invalid registration [").unwrap();
    writer.declaration_action(DeclarationAction::ReloadRegistration);
    finish(&mut writer, &mut jobs);
    assert!(writer.declaration_session().unwrap().registration.is_err());
    assert!(writer.status.contains("Producer registration is invalid"));
    fs::write(
        &registration,
        original.replace("fixture.json", "replacement.json"),
    )
    .unwrap();
    writer.declaration_action(DeclarationAction::ReloadRegistration);
    finish(&mut writer, &mut jobs);
    assert!(
        writer
            .declaration_session()
            .unwrap()
            .registration
            .as_ref()
            .unwrap()
            .is_some()
    );
    assert!(
        writer
            .status
            .contains("Review the command before regenerating")
    );
    writer.show_declaration_producer();
    let labels = support::labels(&mut writer).unwrap();
    assert!(
        labels
            .iter()
            .any(|label| label.contains("replacement.json"))
    );
    assert!(
        labels
            .iter()
            .any(|label| label == "Reload producer registration")
    );
}
