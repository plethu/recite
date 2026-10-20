use super::*;
use crate::{editing::Writer, recovery::status::RecoveryStatus, tests::support};
use std::{fs, path::PathBuf};

const SOURCE: &str =
    ":: start default\n> line@11111111111111111111\n  Hello.\n-> old\n\n:: old\n-> END\n";

fn stage(mut writer: Writer, root: &std::path::Path) -> Vec<u8> {
    let mut files = ProjectFiles::open(root).unwrap();
    let mut model = files.workbench().unwrap();
    model.select(View::Passage(support::A.into())).unwrap();
    files.review_rename(&mut model, "old", "new").unwrap();
    fs::create_dir(root.join("scene.recite.recite-editor-recovery.json")).unwrap();
    assert!(files.apply_rename(&mut model).is_err());
    assert!(files.project_edit_recovery_pending());
    writer.buffers.install(model, false);
    writer.files.set(Some(files));
    writer.buffers.prose.set("Late queued input".into());
    fs::read(root.join(".recite-manifest-draft.json")).unwrap()
}

fn protected_recovery() -> impl IntoElement {
    use_init_theme(light_theme);
    let root = consume_root_context::<PathBuf>();
    let writer = support::writer();
    let journal = use_hook({
        let root = root.clone();
        move || stage(writer, &root)
    });
    crate::buffers::observe_draft(
        writer.buffers.model,
        writer.buffers.editor,
        writer.buffers.prose,
        writer.files,
    );
    let mut status = use_state(String::new);
    rect()
        .width(Size::fill())
        .padding(24.)
        .spacing(16.)
        .child(label().text("Recite Writer · scene.recite"))
        .child(RecoveryStatus {
            files: writer.files,
            retry: EventHandler::new(move |()| crate::recovery::retry_project_edit(writer)),
        })
        .child(
            crate::design::Button::new()
                .named("Check and retry recovery")
                .child("Check and retry recovery")
                .on_press(move |_| {
                    verify_and_retry(writer, &root, &journal);
                    status.set("Recovery boundary held".into());
                }),
        )
        .child(label().text(status.read().clone()))
}

fn verify_and_retry(writer: Writer, root: &std::path::Path, journal: &[u8]) {
    let input = InputValidator::new("Queued direct validator input".into());
    crate::prose::validate(
        writer,
        &View::Passage(support::A.into()),
        AccessibilityId::new_unique(),
        input.clone(),
    );
    assert!(!input.is_valid());
    writer.buffers.harvest();
    {
        let model = writer.buffers.model.peek();
        let model = model.as_ref().unwrap();
        assert_eq!(model.draft(), "Hello.");
        assert!(!model.has_draft());
        assert!(model.document().source().contains(":: new"));
    }
    assert!(writer.try_perform(|_| panic!("editing excluded")).is_err());
    assert!(crate::editor_work::edit(writer, recite_writer_model::EditOperation::Undo).is_err());
    assert!(writer.require_project_edit_ready().is_err());
    assert!(!writer.work.busy());
    flush_recovery(writer.buffers, writer.files, writer.localisation).unwrap();
    assert_eq!(
        fs::read(root.join(".recite-manifest-draft.json")).unwrap(),
        journal
    );
    assert_eq!(
        fs::read_to_string(root.join("scene.recite")).unwrap(),
        SOURCE
    );

    fs::remove_dir(root.join("scene.recite.recite-editor-recovery.json")).unwrap();
    crate::recovery::retry_project_edit(writer);
    writer.require_project_edit_ready().unwrap();
    writer
        .try_perform(|model| {
            model.set_draft("After retry".into());
            Ok(())
        })
        .unwrap();
    assert_eq!(
        writer.buffers.model.peek().as_ref().unwrap().draft(),
        "After retry"
    );
}

#[test]
fn grouped_recovery_excludes_observer_harvest_actions_and_work_until_retry() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("recite.project.toml"),
        "format_version=1\n",
    )
    .unwrap();
    fs::write(root.path().join("scene.recite"), SOURCE).unwrap();
    let path = root.path().to_owned();
    let mut test = TestingRunner::new(
        protected_recovery,
        (900., 650.).into(),
        move |runner| {
            runner.provide_root_context(move || path);
        },
        1.,
    )
    .0;
    if let Ok(path) = std::env::var("RECITE_CAPTURE_FREYA_RECOVERY") {
        test.render_to_file(path);
    }
    let area = test
        .find(|node, element| {
            Rect::try_downcast(element)
                .filter(|rect| {
                    rect.accessibility.builder.label() == Some("Check and retry recovery")
                })
                .map(|_| node.layout().area)
        })
        .unwrap();
    test.click_cursor(area.center().to_f64());
    test.sync_and_update();
    assert!(
        test.find(|_, element| Label::try_downcast(element)
            .filter(|label| label.text.as_ref() == "Recovery boundary held"))
            .is_some()
    );
}
