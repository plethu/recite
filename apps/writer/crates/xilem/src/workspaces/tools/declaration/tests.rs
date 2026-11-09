#![allow(unused_imports)]
use super::super::super::{ContextUpdate, declarations::producer_description, text};
use super::super::tests::{finish, fixture};
use super::super::*;
use super::*;
use crate::Phase;
use crate::external_editor::Request as EditorRequest;
use crate::{tests::support, workspaces::Screen};
use recite_writer_session::declarations::DeclarationAction;
use recite_writer_session::declarations::SourceStatus;
use std::fs;
use tokio::sync::mpsc;
#[path = "metadata/tests.rs"]
pub mod metadata;

#[test]
fn source_binding_keeps_close_guarded_until_the_same_worker_finishes_retirement() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.change(|model| {
        model.set_draft("Retained manuscript field.".into());
        Ok(())
    });
    writer.show_declaration_source();
    writer.declaration_action(DeclarationAction::Bind(
        directory.path().join("schema.toml"),
    ));
    let completed = jobs.try_recv().unwrap().execute();
    let request = completed.request.clone();
    let retirement = writer.accept_tool(completed);
    assert!(writer.project_jobs_busy());
    assert!(writer.declaration_session().unwrap().busy());
    writer.return_to_writing();
    writer.request_close();
    assert!(writer.phase() == Phase::Writing);
    let terminal = retirement.finish(request).unwrap();
    assert!(writer.project_jobs_busy());
    writer.accept_tool(terminal);
    assert!(!writer.project_jobs_busy());
    assert!(writer.workspaces.screen == Screen::Write);
    assert_eq!(writer.session.model().draft(), "Retained manuscript field.");
    assert_eq!(
        writer
            .declaration_session()
            .unwrap()
            .source
            .as_ref()
            .unwrap()
            .path,
        directory.path().join("schema.toml")
    );
    assert!(writer.workspaces.declarations.source.exported().is_none());
}

#[test]
fn checked_artifact_import_and_declaration_save_preserve_the_unapplied_manuscript() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.declaration_action(DeclarationAction::Bind(
        directory.path().join("schema.toml"),
    ));
    finish(&mut writer, &mut jobs);
    writer.show_declaration_source();
    writer.change(|model| {
        model.set_draft("Unapplied manuscript.".into());
        Ok(())
    });
    writer
        .declaration_session_mut()
        .unwrap()
        .source
        .as_mut()
        .unwrap()
        .set_draft("Recovered unfinished [".into())
        .unwrap();
    writer.declaration_action(DeclarationAction::ExportDraft);
    finish(&mut writer, &mut jobs);
    let artifact = writer
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
    let original = fs::read_to_string(directory.path().join("schema.toml")).unwrap();
    fs::write(&artifact, original.replace("Mara", "Repaired")).unwrap();
    writer.declaration_action(DeclarationAction::ImportDraft(artifact.clone()));
    finish(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().draft(), "Unapplied manuscript.");
    assert_eq!(
        fs::read_to_string(directory.path().join("schema.toml")).unwrap(),
        original
    );
    let (sender, mut saves) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.save_declarations();
    support::finish_work(&mut writer, &mut saves).unwrap();
    finish(&mut writer, &mut jobs);
    assert!(
        fs::read_to_string(directory.path().join("schema.json"))
            .unwrap()
            .contains("Repaired")
    );
    assert!(
        fs::read_to_string(directory.path().join("schema.toml"))
            .unwrap()
            .contains("Repaired")
    );
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Hello.")
    );
    assert_eq!(writer.session.model().draft(), "Unapplied manuscript.");
    assert!(!writer.declaration_session().unwrap().dirty());
    assert!(artifact.is_file());
    let labels = support::labels(&mut writer).unwrap();
    assert!(
        labels
            .iter()
            .any(|label| label.contains("Show source draft preview"))
    );
    assert!(writer.declaration_session().unwrap().current());
}

#[test]
fn source_dispatch_failure_releases_both_domain_and_transport_activity() {
    let (directory, mut writer, _jobs) = fixture();
    writer.workspaces.tools.sender = None;
    writer.declaration_action(DeclarationAction::Bind(
        directory.path().join("schema.toml"),
    ));
    assert!(!writer.project_jobs_busy());
    assert!(!writer.declaration_session().unwrap().busy());
    assert!(writer.declaration_session().unwrap().source.is_none());
    assert!(writer.status.contains("worker stopped"));
}
#[path = "visual/tests.rs"]
mod visual;
#[path = "workflow/tests.rs"]
mod workflow;
