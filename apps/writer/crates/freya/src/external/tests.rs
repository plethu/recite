use super::*;
use crate::tests::support;
use freya_testing::prelude::*;
use recite_writer_session::project::ProjectFiles;
use std::{fs, path::PathBuf};

fn comparison() -> impl IntoElement {
    use_init_theme(|| crate::palette::theme(false));
    let root = consume_root_context::<PathBuf>();
    let mut exposed = consume_root_context::<State<Option<Writer>>>();
    let mut writer = support::writer();
    use_hook(move || {
        let mut files = ProjectFiles::open(&root).unwrap();
        let mut model = files.workbench().unwrap();
        model
            .select(recite_writer_model::View::Passage(support::A.into()))
            .unwrap();
        model.set_draft("Pending manuscript text.".into());
        writer.buffers.install(model, false);
        fs::write(
            &files.current,
            support::SOURCE.replace("Before A.", "External text."),
        )
        .unwrap();
        writer.files.set(Some(files));
        try_open(writer).unwrap();
        exposed.set(Some(writer));
    });
    rect()
        .width(Size::fill())
        .height(Size::fill())
        .background(crate::palette::current().canvas)
        .child(ExternalScreen { writer })
}
fn click(test: &mut TestingRunner, caption: &str) {
    let id = test
        .find(|_, element| {
            let data = element.accessibility();
            (data.builder.label() == Some(caption))
                .then_some(data.a11y_id)
                .flatten()
        })
        .unwrap_or_else(|| panic!("missing {caption}"));
    assert!(
        test.send_accessibility_action(AccessibilityActionEventData {
            action: AccessibilityAction::Click,
            target_tree: accesskit::TreeId::ROOT,
            target_node: id,
            data: None
        })
    );
    test.sync_and_update();
}
#[test]
fn comparison_retains_pending_prose_and_synchronizes_the_selected_adoption() {
    for (keep, failed_recovery) in [(true, false), (false, false), (true, true), (false, true)] {
        let root = tempfile::tempdir().unwrap();
        fs::write(
            root.path().join("recite.project.toml"),
            "format_version=1\n",
        )
        .unwrap();
        fs::write(root.path().join("scene.recite"), support::SOURCE).unwrap();
        let path = root.path().to_owned();
        let (mut test, exposed) = TestingRunner::new(
            comparison,
            (1280., 800.).into(),
            move |runner| {
                runner.provide_root_context(move || path);
                runner.provide_root_context(|| State::create(None::<Writer>))
            },
            1.,
        );
        let mut writer = exposed.peek().unwrap();
        assert_eq!(
            writer.buffers.model.peek().as_ref().unwrap().draft(),
            "Pending manuscript text."
        );
        if keep
            && !failed_recovery
            && let Ok(path) = std::env::var("RECITE_CAPTURE_FREYA_EXTERNAL")
        {
            test.render_to_file(path);
        }
        click(
            &mut test,
            if keep {
                "Keep Writer version"
            } else {
                "Use inspected disk version"
            },
        );
        if failed_recovery {
            writer
                .files
                .write()
                .as_mut()
                .unwrap()
                .checkpoint(writer.buffers.model.peek().as_ref().unwrap())
                .unwrap();
            let recovery = root.path().join("scene.recite.recite-editor-recovery.json");
            if recovery.is_file() {
                fs::remove_file(&recovery).unwrap();
            }
            fs::create_dir(recovery).unwrap();
        }
        click(&mut test, "Confirm inspected version");
        let writer = exposed.peek().unwrap();
        let expected = if keep {
            "Pending manuscript text."
        } else {
            "External text."
        };
        assert_eq!(
            writer.buffers.model.peek().as_ref().unwrap().draft(),
            expected
        );
        assert_eq!(&*writer.buffers.prose.peek(), expected);
        assert!(writer.files.peek().as_ref().unwrap().external.is_none());
        assert_eq!(writer.message.is_error(), failed_recovery);
        assert_eq!(
            writer.buffers.model.peek().as_ref().unwrap().view(),
            &recite_writer_model::View::Passage(support::A.into())
        );
        assert!(
            fs::read_to_string(root.path().join("scene.recite"))
                .unwrap()
                .contains("External text.")
        );
    }
}
