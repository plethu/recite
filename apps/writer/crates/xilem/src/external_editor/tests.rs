use super::*;
use crate::tests::support::{self, finish_work, open, project};
use recite_writer_session::project::save::sequence::Work;
use tokio::sync::mpsc as async_mpsc;
use xilem::masonry::accesskit;

#[test]
fn default_editor_retry_keeps_the_repair_artifact_target_and_import_guidance() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let document = writer
        .session
        .project()
        .unwrap()
        .current()
        .path()
        .to_owned();
    let source = directory.path().join("recite-source-repair-example.txt");
    std::fs::write(&source, "Recovered draft").unwrap();
    writer.repair.artifact = Some(recite_writer_session::project::SourceRepairArtifact {
        document: document.clone(),
        source: source.clone(),
        recovery: directory.path().join("backup.json"),
    });
    let (sender, mut launches) = async_mpsc::unbounded_channel();
    writer.external.sender = Some(sender);
    writer.open_exported_source();
    let request = launches.try_recv().unwrap();
    writer.accept_editor(request, Some(Err("missing editor".into())));
    writer.open_system_editor();
    let request = launches.try_recv().unwrap();
    assert_eq!(request.path(), source);
    assert!(request.is_current(writer.session.project().unwrap()));
    assert!(request.editor.is_none());
    writer.accept_editor(request, Some(Ok(())));
    assert!(writer.status.contains("Import repaired source"));
    assert!(!writer.status.contains("Reload to inspect"));
}

#[test]
fn declaration_editor_targets_follow_the_project_while_dialogue_targets_follow_the_document() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let other_path = directory.path().join("other.recite");
    std::fs::write(&other_path, ":: other default\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    let project = writer.session.project().unwrap();
    let source = Request::source(project.current().path().to_owned(), None);
    let draft = Request::declaration_draft(project, directory.path().join("draft.toml"), None);
    writer
        .session
        .project_mut()
        .unwrap()
        .switch(&other_path)
        .unwrap();
    assert!(draft.is_current(writer.session.project().unwrap()));
    assert!(!source.is_current(writer.session.project().unwrap()));
    let foreign = self::project(recite_writer_model::FIXTURE).unwrap();
    let other = open(&foreign).unwrap();
    assert!(!draft.is_current(other.session.project().unwrap()));
    assert!(
        draft
            .next_step()
            .contains("Import repaired declaration draft")
    );
}

#[test]
fn ordinary_default_editor_retry_still_runs_the_checked_source_save() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let (sender, mut saves) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    let (sender, mut launches) = async_mpsc::unbounded_channel();
    writer.external.sender = Some(sender);
    writer.open_editor();
    finish_work(&mut writer, &mut saves).unwrap();
    let request = launches.try_recv().unwrap();
    writer.accept_editor(request, Some(Err("missing editor".into())));
    let source = writer
        .session
        .project()
        .unwrap()
        .current()
        .path()
        .to_owned();
    std::fs::write(&source, "external source change").unwrap();
    writer.open_system_editor();
    finish_work(&mut writer, &mut saves).unwrap();
    assert!(launches.try_recv().is_err());
    assert_eq!(
        std::fs::read_to_string(source).unwrap(),
        "external source change"
    );
    assert!(writer.status.contains("changed on disk"));
}

#[test]
fn accepted_source_is_saved_and_newer_field_typing_stays_open_before_launch() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let path = writer
        .session
        .project()
        .unwrap()
        .current()
        .path()
        .to_owned();
    writer.change(|model| {
        model.set_draft("Accepted before handoff.".into());
        model.apply()
    });
    let accepted = writer.session.model().document().source().to_owned();
    writer.change(|model| {
        model.set_draft("Unfinished field.".into());
        Ok(())
    });
    let (save_sender, mut saves) = async_mpsc::unbounded_channel();
    let (editor_sender, mut launches) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(save_sender);
    writer.external.sender = Some(editor_sender);
    writer.editor = Some(EditorCommand {
        program: "literal editor".into(),
        args: vec!["--wait".into(), "$(not-a-shell)".into()],
    });
    writer.open_editor();
    assert!(writer.work.busy());
    assert!(launches.try_recv().is_err(), "no launch before persistence");
    let work = saves.try_recv().unwrap();
    assert!(
        matches!(work, Work::Write(_)),
        "handoff never applies the field draft"
    );
    let receipt = work.execute();
    writer.change(|model| {
        model.set_draft("Newer typing during handoff.".into());
        Ok(())
    });
    writer.accept_save_work(receipt);
    finish_work(&mut writer, &mut saves).unwrap();
    let request = launches.try_recv().unwrap();
    assert_eq!(request.path(), path);
    assert_eq!(
        request.editor.as_ref().unwrap().args,
        ["--wait", "$(not-a-shell)"].map(std::ffi::OsString::from)
    );
    assert_eq!(std::fs::read_to_string(&path).unwrap(), accepted);
    assert_eq!(
        writer.session.model().draft(),
        "Newer typing during handoff."
    );
    assert!(writer.external.busy());
    writer.open_editor();
    assert!(
        launches.try_recv().is_err(),
        "one independently owned editor at a time"
    );
    writer.accept_editor(request, Some(Err("missing executable".into())));
    assert!(!writer.external.busy());
    assert!(
        writer
            .status
            .starts_with("Could not start the configured editor")
    );
    writer.close_with_recovery();
    drop(writer);
    let reopened = open(&directory).unwrap();
    assert_eq!(
        reopened.session.model().draft(),
        "Newer typing during handoff."
    );
}

#[test]
fn disk_conflict_prevents_launch_and_preserves_the_unapplied_draft() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let (save_sender, mut saves) = async_mpsc::unbounded_channel();
    let (editor_sender, mut launches) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(save_sender);
    writer.external.sender = Some(editor_sender);
    writer.change(|model| {
        model.set_draft("Keep the field draft.".into());
        Ok(())
    });
    let path = writer
        .session
        .project()
        .unwrap()
        .current()
        .path()
        .to_owned();
    std::fs::write(&path, ":: externally_changed default\n-> END\n").unwrap();
    writer.open_editor();
    finish_work(&mut writer, &mut saves).unwrap();
    assert!(launches.try_recv().is_err());
    assert!(!writer.external.busy());
    assert!(writer.status.contains("changed on disk"));
    assert_eq!(writer.session.model().draft(), "Keep the field draft.");
}

#[test]
fn cancelled_launch_still_reconciles_the_dispatched_save() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Accepted source.".into());
        model.apply()
    });
    let accepted = writer.session.model().document().source().to_owned();
    let (save_sender, mut saves) = async_mpsc::unbounded_channel();
    let (editor_sender, mut launches) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(save_sender);
    writer.external.sender = Some(editor_sender);
    writer.open_editor();
    assert!(writer.work.preparing_editor());
    writer.cancel_editor_launch();
    assert!(!writer.work.preparing_editor());
    finish_work(&mut writer, &mut saves).unwrap();
    assert!(launches.try_recv().is_err());
    assert_eq!(
        std::fs::read_to_string(writer.session.project().unwrap().current().path()).unwrap(),
        accepted
    );
    assert!(!writer.work.busy());
}

#[test]
fn editor_completion_is_attributed_to_its_document_after_navigation() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    std::fs::write(&other, ":: other default\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    let (save_sender, mut saves) = async_mpsc::unbounded_channel();
    let (editor_sender, mut launches) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(save_sender);
    writer.external.sender = Some(editor_sender);
    writer.open_editor();
    finish_work(&mut writer, &mut saves).unwrap();
    let request = launches.try_recv().unwrap();
    let (sender, mut navigation) = async_mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.switch(&other);
    support::finish_navigation(&mut writer, &mut navigation).unwrap();
    writer.accept_editor(request, Some(Ok(())));
    assert!(!writer.external.busy());
    assert_eq!(
        writer.session.model().document().key().as_str(),
        "other.recite"
    );
    assert!(writer.status.contains("scene.recite"));
    assert!(!writer.status.contains("Reload to"));
}

#[test]
fn pending_editor_save_keeps_manuscript_enabled_and_exposes_cancellation() {
    use crate::tests::support::NoProxy;
    use xilem::{
        ViewCtx,
        core::View,
        masonry::{core::NewWidget, testing::TestHarness, widgets::SizedBox},
    };
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let (save_sender, mut saves) = async_mpsc::unbounded_channel();
    let (editor_sender, mut launches) = async_mpsc::unbounded_channel();
    writer.work.sender = Some(save_sender);
    writer.external.sender = Some(editor_sender);
    writer.open_editor();
    let mut ctx = ViewCtx::new(
        Arc::new(NoProxy),
        Arc::new(tokio::runtime::Runtime::new().unwrap()),
    );
    let view = crate::view::writer(&writer);
    let save_sender = writer.work.sender.clone();
    let editor_sender = writer.external.sender.clone();
    let (pod, _) = view.build(&mut ctx, &mut writer);
    writer.work.sender = save_sender;
    writer.external.sender = editor_sender;
    let properties = crate::presentation::Theme::new(&writer.config.writer).initial_properties();
    let mut harness = TestHarness::create_with_size(
        properties,
        NewWidget::new(SizedBox::new(pod.new_widget)),
        (1280, 700),
    );
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut cancel = false;
    let mut editable = false;
    while let Some(node) = pending.pop() {
        cancel |= node.label().as_deref() == Some("Cancel editor launch");
        editable |= node.role() == accesskit::Role::MultilineTextInput && !node.is_disabled();
        pending.extend(node.children());
    }
    assert!(cancel && editable);
    if let Ok(path) = std::env::var("RECITE_CAPTURE_EDITOR_WAIT") {
        harness.render().save(path).unwrap();
    }
    writer.cancel_editor_launch();
    finish_work(&mut writer, &mut saves).unwrap();
    assert!(launches.try_recv().is_err());
}
