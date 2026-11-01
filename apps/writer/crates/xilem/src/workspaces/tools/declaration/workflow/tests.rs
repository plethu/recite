use super::*;
use crate::keyed::keyed;
use crate::{
    external_editor::tests::Driver as EditorDriver, tests::support::mounted::Mounted,
    workspaces::tools::tests::Driver as ToolDriver,
};
use recite_writer_session::startup::EditorCommand;
use xilem::WidgetView;
use xilem::masonry::widgets::Passthrough;

fn surface(writer: &mut Writer) -> impl WidgetView<Writer, Widget = Passthrough> + use<> {
    keyed((), crate::view::writer(writer))
}

#[test]
fn mounted_declaration_repair_uses_its_artifact_through_editor_retry_import_and_save() {
    let (directory, mut writer, _) = fixture();
    writer.show_declaration_source();
    writer.change(|model| {
        model.set_draft("Unapplied manuscript.".into());
        Ok(())
    });
    let source = directory.path().join("schema.toml");
    let original = fs::read_to_string(&source).unwrap();
    let mut mounted = Mounted::new(writer, surface);
    let mut jobs = ToolDriver::new(&mut mounted.state);
    mounted.type_text("Declaration source path", source.to_str().unwrap());
    mounted.activate("Bind standalone source");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    let mut editor = EditorDriver::new(&mut mounted.state);
    mounted.state.editor = Some(EditorCommand {
        program: "missing editor".into(),
        args: vec!["literal argument".into()],
    });
    mounted.activate("Open saved declaration source in editor");
    editor.complete(&mut mounted.state, &source, Ok(()));
    assert!(mounted.state.status.contains("Reload declaration source"));
    mounted
        .state
        .declaration_session_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .set_draft("Recovered unfinished [".into())
        .unwrap();
    mounted.state.refresh_declaration_rows();
    jobs.finish(&mut mounted.state);
    mounted.settle();
    mounted.activate("Export declaration draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    let artifact = mounted
        .state
        .workspaces
        .declarations
        .source
        .exported()
        .unwrap()
        .to_owned();
    assert_eq!(
        fs::read_to_string(&artifact).unwrap(),
        "Recovered unfinished ["
    );
    mounted.activate("Open exported declaration draft in editor");
    editor.complete(
        &mut mounted.state,
        &artifact,
        Err("missing configured editor".into()),
    );
    assert!(mounted.state.status.contains("configured editor"));
    mounted.state.open_system_editor();
    editor.complete(&mut mounted.state, &artifact, Ok(()));
    assert!(
        mounted
            .state
            .status
            .contains("Import repaired declaration draft")
    );
    fs::write(&artifact, original.replace("Mara", "Repaired")).unwrap();
    mounted.settle();
    mounted.activate("Import repaired declaration draft");
    jobs.finish(&mut mounted.state);
    mounted.settle();
    assert_eq!(fs::read_to_string(&source).unwrap(), original);
    assert!(
        mounted
            .state
            .workspaces
            .declarations
            .source
            .exported()
            .is_none()
    );
    let (sender, mut saves) = mpsc::unbounded_channel();
    mounted.state.work.sender = Some(sender);
    mounted.activate("Save and generate declarations");
    support::finish_work(&mut mounted.state, &mut saves).unwrap();
    jobs.finish(&mut mounted.state);
    assert!(fs::read_to_string(&source).unwrap().contains("Repaired"));
    assert!(
        fs::read_to_string(directory.path().join("schema.json"))
            .unwrap()
            .contains("Repaired")
    );
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Hello.")
    );
    assert_eq!(
        mounted.state.session.model().draft(),
        "Unapplied manuscript."
    );
    assert!(!mounted.state.project_jobs_busy());
}

#[test]
fn cancelling_a_prepared_source_replacement_keeps_its_archive_and_releases_the_recovery_fence() {
    let (directory, mut writer, mut jobs) = fixture();
    let source = directory.path().join("schema.toml");
    writer.declaration_action(DeclarationAction::Bind(source.clone()));
    finish(&mut writer, &mut jobs);
    writer
        .declaration_session_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .set_draft("Retain unfinished [".into())
        .unwrap();
    let replacement = fs::read_to_string(&source)
        .unwrap()
        .replace("Mara", "External");
    fs::write(&source, &replacement).unwrap();
    writer.declaration_action(DeclarationAction::ReloadSource);
    let prepared = jobs.try_recv().unwrap().execute();
    writer.cancel_declaration_action();
    let request = prepared.request.clone();
    let retirement = writer.accept_tool(prepared);
    if let Some(terminal) = retirement.finish(request) {
        writer.accept_tool(terminal);
    }
    writer.poll_project_jobs();
    assert!(!writer.project_jobs_busy());
    assert!(!writer.declaration_session().unwrap().busy());
    assert_eq!(
        writer
            .declaration_session()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .draft,
        "Retain unfinished ["
    );
    writer.show_declaration_source();
    let labels = support::labels(&mut writer).unwrap();
    let archive = labels
        .iter()
        .find_map(|label| label.strip_prefix("Previous draft preserved at "))
        .expect("the prepared recovery artifact must remain discoverable");
    assert!(
        fs::read_to_string(archive)
            .unwrap()
            .contains("Retain unfinished [")
    );
    let mut driver = ToolDriver::new(&mut writer);
    writer.declaration_action(DeclarationAction::ReloadSource);
    driver.finish(&mut writer);
    assert_eq!(
        writer
            .declaration_session()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .draft,
        replacement
    );
    assert!(!writer.project_jobs_busy());
}
