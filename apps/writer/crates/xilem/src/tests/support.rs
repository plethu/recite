use super::*;
use masonry::testing::TestHarness;
use std::fs;
use std::sync::Arc;
use xilem::ViewCtx;
use xilem::core::{
    DynMessage, MessageCtx, ProxyError, RawProxy, SendMessage, View as _, ViewId, ViewPathTracker,
};

pub(crate) fn project(source: &str) -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"test\"\nversion = \"1\"\n",
    )?;
    fs::write(directory.path().join("scene.recite"), source)?;
    Ok(directory)
}

pub(crate) fn open(directory: &tempfile::TempDir) -> Result<Writer, Box<dyn std::error::Error>> {
    Ok(Writer::open(Startup {
        project: directory.path().to_owned(),
        document: Some("scene.recite".into()),
        editor: None,
        editor_args: vec![],
    })?)
}

#[derive(Debug)]
pub(crate) struct NoProxy;
impl RawProxy for NoProxy {
    fn send_message(&self, _path: Arc<[ViewId]>, message: SendMessage) -> Result<(), ProxyError> {
        Err(ProxyError::DriverFinished(message))
    }
    fn dyn_debug(&self) -> &dyn std::fmt::Debug {
        self
    }
}

pub(crate) fn labels(writer: &mut Writer) -> Result<Vec<String>, Box<dyn std::error::Error>> {
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

// Drive actual view callbacks through the same message path as Xilem's window driver.
pub(crate) fn click(writer: &mut Writer, names: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    let mut context = ViewCtx::new(Arc::new(NoProxy), Arc::new(tokio::runtime::Runtime::new()?));
    let mut view = view::writer(writer);
    let (pod, mut state) = view.build(&mut context, writer);
    let (sender, mut requests) = tokio::sync::mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
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
    if writer.phase == Phase::Writing {
        harness.focus_on(field);
    }
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
        finish_work(writer, &mut requests)?;
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

// Native callbacks schedule owned work; this harness executes it deterministically.
pub(crate) fn finish_work(
    writer: &mut Writer,
    requests: &mut tokio::sync::mpsc::UnboundedReceiver<
        recite_writer_session::project::save::sequence::Work,
    >,
) -> Result<(), Box<dyn std::error::Error>> {
    while writer.work.busy() {
        let work = requests.try_recv()?;
        writer.accept_save_work(work.execute());
    }
    Ok(())
}
pub(crate) fn save(writer: &mut Writer) -> Result<(), Box<dyn std::error::Error>> {
    let (sender, mut requests) = tokio::sync::mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.save();
    finish_work(writer, &mut requests)
}
