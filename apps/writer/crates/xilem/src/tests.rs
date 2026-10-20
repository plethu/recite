use super::*;
use masonry::testing::TestHarness;
use std::fs;
use std::sync::Arc;
use xilem::ViewCtx;
use xilem::core::{
    DynMessage, MessageCtx, ProxyError, RawProxy, SendMessage, View as _, ViewId, ViewPathTracker,
};

fn project(source: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"test\"\nversion = \"1\"\n",
    )?;
    fs::write(directory.path().join("scene.recite"), source)?;
    Ok(directory)
}

fn open(directory: &tempfile::TempDir) -> Result<Writer, Box<dyn std::error::Error>> {
    Ok(Writer::open(Startup {
        project: directory.path().to_owned(),
        document: Some("scene.recite".into()),
        editor: None,
        editor_args: vec![],
    })?)
}

#[test]
fn unsaved_close_requires_a_decision_and_keeps_a_reopenable_draft() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Keep this draft.".into());
        Ok(())
    });
    let selected = writer.session.model().view().clone();
    writer.select("missing".into());
    assert_eq!(writer.session.model().view(), &selected);
    writer.request_close();
    assert!(writer.phase == Phase::ConfirmClose);
    assert!(writer.keep_running());
    writer.close_with_recovery();
    assert!(!writer.keep_running());
    drop(writer);
    let mut writer = open(&directory).unwrap();
    assert_eq!(writer.session.model().draft(), "Keep this draft.");
    assert!(writer.save());
    writer.request_close();
    assert!(!writer.keep_running());
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Keep this draft.")
    );
}

#[test]
fn save_conflict_stays_open_and_reload_preserves_the_local_work() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("My work.".into());
        Ok(())
    });
    fs::write(
        directory.path().join("scene.recite"),
        ":: scene default\n->\n",
    )
    .unwrap();
    assert!(!writer.save());
    assert!(writer.keep_running());
    writer.reload();
    assert_eq!(writer.session.model().view(), &View::Source);
    assert!(writer.status.contains("previous work preserved"));
    writer.reload();
    assert_eq!(writer.status, "Reloaded from disk");
    fs::remove_file(directory.path().join("recite.project.toml")).unwrap();
    writer.reload();
    assert_ne!(writer.status, "Reloaded from disk");
}

#[test]
fn startup_rejects_missing_repeated_and_unpaired_options() {
    for arguments in [
        vec![],
        vec!["--project"],
        vec!["--project", "a", "--project", "b"],
        vec!["--wat", "a"],
        vec!["--project", "a", "--editor-arg", "x"],
    ] {
        assert!(Startup::parse(arguments.into_iter().map(OsString::from)).is_err());
    }
    let args = [
        "--project",
        "a space [x] $test",
        "--document",
        "scene.recite",
        "--editor",
        "editor",
        "--editor-arg",
        "--wait",
    ];
    let startup = Startup::parse(args.into_iter().map(OsString::from)).unwrap();
    assert_eq!(startup.project, PathBuf::from("a space [x] $test"));
    assert_eq!(startup.document, Some("scene.recite".into()));
    assert_eq!(startup.editor_args, [OsString::from("--wait")]);
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    assert!(
        Writer::open(Startup {
            document: Some("missing.recite".into()),
            project: directory.path().to_owned(),
            editor: None,
            editor_args: vec![]
        })
        .is_err()
    );
}

#[derive(Debug)]
struct NoProxy;
impl RawProxy for NoProxy {
    fn send_message(&self, _path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        Err(ProxyError::DriverFinished(message))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

fn labels(writer: &mut Writer) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let runtime = Arc::new(tokio::runtime::Runtime::new()?);
    let mut context = ViewCtx::new(Arc::new(NoProxy), runtime);
    let view = view::writer(writer);
    let (pod, _) = view.build(&mut context, writer);
    let root = masonry::core::NewWidget::new(masonry::widgets::SizedBox::new(pod.new_widget));
    let harness =
        TestHarness::create_with_size(masonry::theme::default_property_set(), root, (900, 650));
    let tree = harness.access_tree();
    let mut pending = vec![tree.state().root()];
    let mut labels = vec![];
    while let Some(node) = pending.pop() {
        if let Some(label) = node.label().or_else(|| node.value()) {
            labels.push(label);
        }
        pending.extend(node.children());
    }
    Ok(labels)
}

#[test]
fn manuscript_and_close_decisions_have_native_accessible_names() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Manuscript")
    );
    writer.change(|model| {
        model.set_draft("Draft".into());
        Ok(())
    });
    writer.request_close();
    let labels = labels(&mut writer).unwrap();
    for name in ["Keep editing", "Save and close", "Close and keep recovery"] {
        assert!(labels.iter().any(|label| label == name), "missing {name}");
    }
}

#[test]
fn malformed_source_offers_external_repair_without_an_editable_manuscript() {
    let directory = project(":: scene default\n->\n").unwrap();
    let mut writer = open(&directory).unwrap();
    let labels = labels(&mut writer).unwrap();
    assert!(!labels.iter().any(|label| label == "Manuscript"));
    assert!(labels.iter().any(|label| label == "Open in editor"));
    assert!(
        labels
            .iter()
            .any(|label| label == "Source editing in your editor")
    );
}

// Drive actual view callbacks through the same message path as Xilem's window driver.
fn click(writer: &mut Writer, names: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = ViewCtx::new(Arc::new(NoProxy), Arc::new(tokio::runtime::Runtime::new()?));
    let mut view = view::writer(writer);
    let (pod, mut state) = view.build(&mut context, writer);
    let mut harness = TestHarness::create_with_size(
        masonry::theme::default_property_set(),
        pod.new_widget,
        (900, 650),
    );
    let mut pending = vec![harness.root_widget().as_dyn()];
    let mut field = None;
    while let Some(widget) = pending.pop() {
        if context.get_id_path(widget.id()).is_some()
            && harness
                .access_node(widget.id())
                .and_then(|node| node.label())
                .as_deref()
                == Some("Manuscript")
        {
            field = Some(widget.id());
            break;
        }
        pending.extend(widget.children());
    }
    harness.focus_on(field);
    for name in names {
        let mut pending = vec![harness.root_widget().as_dyn()];
        let id = loop {
            let widget = pending
                .pop()
                .ok_or_else(|| format!("missing action {name}"))?;
            if context.get_id_path(widget.id()).is_some()
                && harness
                    .access_node(widget.id())
                    .and_then(|node| node.label())
                    .as_deref()
                    == Some(name)
            {
                break widget.id();
            }
            pending.extend(widget.children());
        };
        harness.process_access_event(masonry::accesskit::ActionRequest {
            action: masonry::accesskit::Action::Click,
            target_tree: masonry::accesskit::TreeId::ROOT,
            target_node: id.to_raw().into(),
            data: None,
        });
        let (action, source) = harness
            .pop_action_erased()
            .ok_or_else(|| format!("no action from {name}"))?;
        let path = context
            .get_id_path(source)
            .ok_or("missing view path")?
            .clone();
        let mut message = MessageCtx::new(
            std::mem::take(context.environment()),
            path,
            DynMessage(action),
        );
        harness.edit_root_widget(|root| {
            view.message(&mut state, &mut message, root, writer);
        });
        *context.environment() = message.finish().0;
        let next = view::writer(writer);
        harness
            .edit_root_widget(|root| next.rebuild(&view, &mut state, &mut context, root, writer));
        view = next;
        harness.redraw();
        if *name == "Close" && writer.phase == Phase::ConfirmClose {
            let mut focused = None;
            for _ in 0..3 {
                harness.press_tab_key(false);
                focused = harness
                    .focused_widget_id()
                    .and_then(|id| harness.access_node(id))
                    .and_then(|node| node.label());
                if focused.as_deref() == Some("Keep editing") {
                    break;
                }
            }
            assert_eq!(focused.as_deref(), Some("Keep editing"));
        }
    }
    Ok(())
}

#[test]
fn native_view_actions_apply_discard_undo_and_guard_save() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let original = writer.session.model().draft().to_owned();
    writer.change(|model| {
        model.set_draft("Accepted.".into());
        Ok(())
    });
    click(&mut writer, &["Apply draft", "Undo applied"]).unwrap();
    assert_eq!(writer.session.model().draft(), original);
    click(&mut writer, &["Redo applied"]).unwrap();
    assert_eq!(writer.session.model().draft(), "Accepted.");
    writer.change(|model| {
        model.set_draft("Discard this.".into());
        Ok(())
    });
    click(&mut writer, &["Discard draft", "Close", "Keep editing"]).unwrap();
    assert!(writer.phase == Phase::Writing);
    assert_eq!(writer.session.model().draft(), "Accepted.");
    click(&mut writer, &["Close", "Save and close"]).unwrap();
    assert!(!writer.keep_running());
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Accepted.")
    );
}

#[test]
fn editor_launch_errors_are_visible_and_preserve_the_draft() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.open_editor();
    assert_eq!(writer.status, "Ready");
    writer.editor = Some(directory.path().join("missing-editor").into_os_string());
    writer.change(|model| {
        model.set_draft("Still here.".into());
        Ok(())
    });
    click(&mut writer, &["Open in editor"]).unwrap();
    assert!(writer.status.starts_with("Could not open the editor:"));
    assert_eq!(writer.session.model().draft(), "Still here.");
    click(&mut writer, &["Close", "Close and keep recovery"]).unwrap();
    assert!(!writer.keep_running());
}
